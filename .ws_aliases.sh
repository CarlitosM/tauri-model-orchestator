# Create and checkout a new branch then push it to the remote.
#   Usage: gitB <branch>
#   Example: gitB my_branch
gitB() {
  if [ -z "$1" ]; then
    echo "Usage: gitB <branch>" >&2
    return 1
  fi
  git switch -c "$1"
  git push --set-upstream origin "$1"
}
