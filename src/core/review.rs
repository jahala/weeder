//! The second key: whether a person with write access approved the commit a
//! pull request was judged at.
//!
//! A false block on a pull request is let through by two people's acts. The
//! author states the reason in a `Weeder-allow: <RULE> <reason>` trailer on a
//! commit of the change, and a person with write access accepts it by
//! approving the pull request at its head. An agent cannot approve its own pull
//! request, and a review carries the commit it was given on, so a later push
//! needs a fresh approval. Under `--strict` weeder then honours the trailers of
//! the judged commits, and never an inline marker, which is the change's own
//! line.
//!
//! weeder asks the forge nothing. The caller reads the reviews and each
//! reviewer's permission over the API and hands weeder the answers as the forge
//! gave them; this module decides what they amount to. That keeps the network
//! at the edge, and the decision in one place a test can run on real answers.

use serde::Deserialize;

/// The permissions that may accept a trailer. GitHub's `permission` field folds
/// `maintain` into `write` and `triage` into `read`.
const WRITERS: [&str; 2] = ["admin", "write"];

/// The account kind that is a person. A bot's review accepts nothing, because
/// an agent that runs as an app could otherwise approve its own change.
const PERSON: &str = "User";

/// An approval that accepts the trailers of the judged change: who gave it, in
/// login order, and the commit they approved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Approval {
    pub reviewers: Vec<String>,
    pub commit: String,
}

/// Answers weeder could not read as the forge's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewError {
    reason: String,
}

impl std::fmt::Display for ReviewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "weeder could not read the reviews it was handed: {}. hand it the pull request's head, its reviews and each reviewer's permission as GitHub answered them.",
            self.reason
        )
    }
}

impl std::error::Error for ReviewError {}

/// What the caller hands in: the head commit the reviews were read for, every
/// review of the pull request, and the permission answer of each reviewer.
#[derive(Debug, Deserialize)]
struct Answers {
    head: String,
    reviews: Vec<Review>,
    permissions: Vec<Permission>,
}

#[derive(Debug, Deserialize)]
struct Review {
    id: u64,
    state: String,
    commit_id: Option<String>,
    user: Option<Account>,
}

#[derive(Debug, Deserialize)]
struct Account {
    login: String,
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Debug, Deserialize)]
struct Permission {
    permission: String,
    user: Option<Account>,
}

/// The approval the answers carry for the judged change, if any.
///
/// `judged` names the commits an approval may be of: the commit judged, and,
/// where that is the merge a forge checks a pull request out as, the head it
/// merges. Answers read for any other head accept nothing.
///
/// Each reviewer's word is their newest review that approves, asks for changes
/// or was dismissed; a comment leaves it standing. It accepts where it approves
/// the head, and where the reviewer is a person whose permission is admin or
/// write.
pub fn approval(answers: &str, judged: &[String]) -> Result<Option<Approval>, ReviewError> {
    let answers: Answers = serde_json::from_str(answers).map_err(|error| ReviewError {
        reason: error.to_string(),
    })?;
    if !judged.contains(&answers.head) {
        return Ok(None);
    }

    let writers: Vec<&str> = answers
        .permissions
        .iter()
        .filter(|answer| WRITERS.contains(&answer.permission.as_str()))
        .filter_map(|answer| answer.user.as_ref())
        .filter(|account| account.kind == PERSON)
        .map(|account| account.login.as_str())
        .collect();

    let mut reviews: Vec<&Review> = answers.reviews.iter().collect();
    reviews.sort_by_key(|review| review.id);
    // Keyed by login, so the reviewers come out in one order on every run.
    let mut word: std::collections::BTreeMap<&str, &Review> = std::collections::BTreeMap::new();
    for review in reviews {
        let Some(account) = &review.user else {
            continue;
        };
        if matches!(
            review.state.as_str(),
            "APPROVED" | "CHANGES_REQUESTED" | "DISMISSED"
        ) {
            word.insert(account.login.as_str(), review);
        }
    }

    let reviewers: Vec<String> = word
        .into_iter()
        .filter(|(login, review)| {
            review.state == "APPROVED"
                && review.commit_id.as_deref() == Some(answers.head.as_str())
                && review
                    .user
                    .as_ref()
                    .is_some_and(|account| account.kind == PERSON)
                && writers.contains(login)
        })
        .map(|(login, _)| login.to_string())
        .collect();
    if reviewers.is_empty() {
        return Ok(None);
    }
    Ok(Some(Approval {
        reviewers,
        commit: answers.head,
    }))
}
