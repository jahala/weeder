import { config } from "./settings";

export const region = "eu-west-1";
export const cacheKey = "user-profile";
export const apiKey = "<your-key-here>";
export const uploadToken = "<paste-your-token-here-Xq7vN2mB4dW>";
export const sessionToken = process.env.ANTHROPIC_AUTH_TOKEN;
export const refreshToken = process.env["ANTHROPIC_REFRESH_TOKEN"];
export const authToken = config.credentials.anthropicAuthToken;
export const GITHUB_TOKEN_WARNING = "That doesn't look like a GitHub token. Are you sure?";
