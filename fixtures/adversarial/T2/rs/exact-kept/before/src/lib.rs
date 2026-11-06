/// What to tell an agent to do with its branch.
pub fn instruction(branch: &str, base: &str, remote: &str) -> String {
    format!("Rebase {branch} onto {base} from {remote}.")
}
