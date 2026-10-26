import os

from .settings import settings

REGION = "eu-west-1"
CACHE_KEY = "user-profile"
API_KEY = "<your-key-here>"
UPLOAD_TOKEN = "<paste-your-token-here-Xq7vN2mB4dW>"
SESSION_TOKEN = os.environ["ANTHROPIC_AUTH_TOKEN"]
REFRESH_TOKEN = os.getenv("ANTHROPIC_REFRESH_TOKEN")
AUTH_TOKEN = settings.credentials.ANTHROPIC_AUTH_TOKEN
