# Homebrew Tap for LiteForge

This directory contains template files for the Homebrew tap repository.

## Setup Instructions

1. Create a new repository at `gitea.poyner.ai/sean/homebrew-forge`

2. Copy these files to the new repository:
   ```bash
   # Clone the new empty repo
   git clone https://gitea.poyner.ai/sean/homebrew-forge.git
   cd homebrew-forge

   # Copy template files
   cp -r /path/to/liteforge/homebrew-tap-template/* .

   # Commit and push
   git add .
   git commit -m "Initial Homebrew tap setup"
   git push
   ```

3. Create a Personal Access Token with `repo` scope for the GitHub Actions workflow to update the formula automatically.

4. Add the token as a secret named `HOMEBREW_TAP_TOKEN` in the main liteforge repository.

## Usage

Once the tap is set up, users can install forge-cli with:

```bash
# Add the tap
brew tap sean/forge https://gitea.poyner.ai/sean/homebrew-forge.git

# Install forge-cli
brew install forge-cli

# Or in one command
brew install sean/forge/forge-cli
```

## Formula Updates

The formula is automatically updated by the release workflow in liteforge when a new version is tagged.

To manually update the formula:

1. Calculate SHA256 checksums for the release archives
2. Update the URLs and checksums in `Formula/forge-cli.rb`
3. Commit and push
