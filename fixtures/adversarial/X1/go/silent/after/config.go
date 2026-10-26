package demo

import "os"

const Region = "eu-west-1"
const CacheKey = "user-profile"
const APIKey = "<your-key-here>"
const UploadToken = "<paste-your-token-here-Xq7vN2mB4dW>"

var APIToken = os.Getenv("ANTHROPIC_AUTH_TOKEN")
var RefreshToken, _ = os.LookupEnv("ANTHROPIC_REFRESH_TOKEN")
var AuthToken = config.Credentials.AnthropicAuthToken
