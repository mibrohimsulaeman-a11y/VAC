#!/bin/bash

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CARGO_MANIFEST="$ROOT/vac-rs/Cargo.toml"
CARGO_LOCK="$ROOT/vac-rs/Cargo.lock"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to show usage
show_usage() {
    echo "Usage: $0 [patch|minor|major|<specific_version>] [--beta]"
    echo ""
    echo "Examples:"
    echo "  $0 patch          # Bump patch version (0.3.86-vac.1 -> 0.3.87)"
    echo "  $0 minor          # Bump minor version (0.3.86-vac.1 -> 0.4.0)"
    echo "  $0 major          # Bump major version (0.3.86-vac.1 -> 1.0.0)"
    echo "  $0 1.2.3          # Set specific version to 1.2.3"
    echo "  $0                # Interactive mode - will prompt for version type"
    echo ""
    echo "Beta releases:"
    echo "  $0 patch --beta   # Create beta release (0.3.86-vac.1 -> 0.3.87-beta.1)"
    echo "  $0 --beta         # Interactive mode with beta suffix"
    echo ""
    echo "Note: Beta releases create tags like v0.3.87-beta.1 and push to"
    echo "      a separate branch in homebrew-vac for testing."
}

# Function to get current version from Cargo.toml
get_current_version() {
    grep -m1 '^version = ' "$CARGO_MANIFEST" | sed -E 's/version = "([^"]+)"/\1/'
}

# Function to validate semantic version format
# Accepts: X.Y.Z or X.Y.Z-<semver-prerelease>, for example X.Y.Z-beta.N
# or the repository's current X.Y.Z-vac.N prerelease channel.
validate_version() {
    local version=$1
    if [[ ! $version =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z][0-9A-Za-z.-]*)?$ ]]; then
        print_error "Invalid version format: $version. Expected semver format: X.Y.Z or X.Y.Z-<prerelease>"
        return 1
    fi
    return 0
}

# Function to bump version
bump_version() {
    local current_version=$1
    local bump_type=$2
    local base_version=${current_version%%-*}

    if [[ ! $base_version =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)$ ]]; then
        print_error "Cannot bump non-semver base version: $current_version"
        return 1
    fi

    local major=${BASH_REMATCH[1]}
    local minor=${BASH_REMATCH[2]}
    local patch=${BASH_REMATCH[3]}

    case $bump_type in
        "patch")
            patch=$((patch + 1))
            ;;
        "minor")
            minor=$((minor + 1))
            patch=0
            ;;
        "major")
            major=$((major + 1))
            minor=0
            patch=0
            ;;
        *)
            print_error "Invalid bump type: $bump_type"
            return 1
            ;;
    esac

    echo "$major.$minor.$patch"
}

# Function to update version in Cargo.toml and Cargo.lock
update_cargo_version() {
    local new_version=$1

    VERSION="$new_version" MANIFEST="$CARGO_MANIFEST" python3 - <<'PY_UPDATE_CARGO_VERSION'
import os
import re
from pathlib import Path

version = os.environ["VERSION"]
manifest = Path(os.environ["MANIFEST"])
text = manifest.read_text(encoding="utf-8")

text, workspace_version_count = re.subn(
    r'(^version\s*=\s*")[^"]+(")',
    rf'\g<1>{version}\2',
    text,
    count=1,
    flags=re.MULTILINE,
)
if workspace_version_count != 1:
    raise SystemExit("failed to update [workspace.package] version")

workspace_dependency_version = re.compile(
    r'^(?P<prefix>vac[A-Za-z0-9_-]*\s*=\s*\{[^}\n]*\bversion\s*=\s*")[^"]+(?P<suffix>"[^}\n]*\})',
    re.MULTILINE,
)
text = workspace_dependency_version.sub(
    lambda match: f"{match.group('prefix')}{version}{match.group('suffix')}",
    text,
)

manifest.write_text(text, encoding="utf-8")
PY_UPDATE_CARGO_VERSION

    print_success "Updated workspace version and internal dependency versions to $new_version"

    # Update Cargo.lock to reflect the new version
    print_info "Updating Cargo.lock..."
    if cargo update --manifest-path "$CARGO_MANIFEST" --workspace; then
        print_success "Updated Cargo.lock"
    else
        print_error "Failed to update Cargo.lock"
        exit 1
    fi
}

# Function to check if git working directory is clean
check_git_status() {
    if [[ -n $(git status --porcelain) ]]; then
        print_warning "Working directory has uncommitted changes."
        echo "The following files will be included in the release commit:"
        git status --short
        echo ""
        read -p "Do you want to continue? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            print_info "Release cancelled."
            exit 0
        fi
    fi
}

# Function to commit and push changes
commit_and_push() {
    local version=$1

    print_info "Adding changes to git..."
    git add "$CARGO_MANIFEST" "$CARGO_LOCK"

    # Add any other uncommitted changes if they exist
    if [[ -n $(git status --porcelain) ]]; then
        git add .
    fi

    print_info "Committing version bump..."
    git commit -m "chore: bump version to $version"

    print_info "Pushing changes to remote..."
    git push origin $(git branch --show-current)

    print_success "Changes committed and pushed"
}

# Function to create and push git tag
create_and_push_tag() {
    local version=$1
    local tag="v$version"

    print_info "Creating git tag: $tag"
    git tag "$tag"

    print_info "Pushing tag to remote..."
    git push --tags

    print_success "Tag $tag created and pushed"
}

# Function to get next beta number for a version
get_next_beta_number() {
    local base_version=$1
    local latest_beta=$(git tag -l "v${base_version}-beta.*" | sort -V | tail -1)

    if [[ -z "$latest_beta" ]]; then
        echo "1"
    else
        local current_beta_num=$(echo "$latest_beta" | sed -E 's/.*-beta\.([0-9]+)/\1/')
        echo $((current_beta_num + 1))
    fi
}

# Main script logic
main() {
    cd "$ROOT"

    print_info "Starting release process..."

    # Parse arguments for --beta flag
    local is_beta=false
    local version_input=""

    for arg in "$@"; do
        if [[ "$arg" == "--beta" ]]; then
            is_beta=true
        elif [[ -z "$version_input" ]]; then
            version_input="$arg"
        fi
    done

    # Check if we're in a git repository
    if ! git rev-parse --git-dir > /dev/null 2>&1; then
        print_error "Not in a git repository"
        exit 1
    fi

    # Check if the Cargo workspace manifest exists
    if [[ ! -f "$CARGO_MANIFEST" ]]; then
        print_error "Cargo workspace manifest not found: $CARGO_MANIFEST"
        exit 1
    fi

    # Get current version (strip any existing beta suffix for base version)
    current_version=$(get_current_version)
    base_version=${current_version%%-*}

    if [[ -z "$current_version" ]]; then
        print_error "Could not find version in Cargo.toml"
        exit 1
    fi

    print_info "Current version: $current_version"
    if [[ "$is_beta" == true ]]; then
        print_info "Beta release mode enabled"
    fi

    # Determine new version
    local new_version

    if [[ -z "$version_input" ]]; then
        # Interactive mode
        echo ""
        echo "Select version bump type:"
        echo "1) patch (${base_version} -> $(bump_version "$base_version" "patch"))"
        echo "2) minor (${base_version} -> $(bump_version "$base_version" "minor"))"
        echo "3) major (${base_version} -> $(bump_version "$base_version" "major"))"
        echo "4) custom (specify exact version)"
        echo ""
        read -p "Enter choice (1-4): " -n 1 -r choice
        echo ""

        case $choice in
            1) new_version=$(bump_version "$base_version" "patch") ;;
            2) new_version=$(bump_version "$base_version" "minor") ;;
            3) new_version=$(bump_version "$base_version" "major") ;;
            4)
                read -p "Enter custom semver version (X.Y.Z or X.Y.Z-<prerelease>): " custom_version
                if validate_version "$custom_version"; then
                    new_version="$custom_version"
                else
                    exit 1
                fi
                ;;
            *)
                print_error "Invalid choice"
                exit 1
                ;;
        esac
    elif [[ "$version_input" == "patch" || "$version_input" == "minor" || "$version_input" == "major" ]]; then
        # Bump version based on type
        new_version=$(bump_version "$base_version" "$version_input")
    elif validate_version "$version_input"; then
        # Specific version provided
        new_version="$version_input"
    else
        show_usage
        exit 1
    fi

    # Add beta suffix if --beta flag is set
    if [[ "$is_beta" == true ]]; then
        if [[ "$new_version" == *-* ]]; then
            print_error "--beta cannot be combined with an explicit prerelease version: $new_version"
            exit 1
        fi
        beta_num=$(get_next_beta_number "$new_version")
        new_version="${new_version}-beta.${beta_num}"
        print_info "Beta version: $new_version"
    fi

    print_info "New version will be: $new_version"

    # Confirm the release
    echo ""
    if [[ "$is_beta" == true ]]; then
        read -p "Proceed with BETA release $current_version -> $new_version? (y/N): " -n 1 -r
    else
        read -p "Proceed with release $current_version -> $new_version? (y/N): " -n 1 -r
    fi
    echo ""
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        print_info "Release cancelled."
        exit 0
    fi

    # Check git status
    check_git_status

    # Update version in Cargo.toml
    update_cargo_version "$new_version"

    # Commit and push changes
    commit_and_push "$new_version"

    # Create and push tag
    create_and_push_tag "$new_version"

    if [[ "$is_beta" == true ]]; then
        print_success "Beta release $new_version completed successfully! 🧪"
        print_info "Install beta from GitHub release:"
        print_info "  curl -L https://github.com/Vastar-AI/vac/releases/download/v${new_version}/vac-darwin-aarch64.tar.gz | tar xz"
        print_info "  sudo mv vac /usr/local/bin/"
    else
        print_success "Release $new_version completed successfully! 🎉"
    fi
    print_info "You can now check your CI/CD pipeline or manually trigger any additional release processes."
}

# Handle help flag
if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
    show_usage
    exit 0
fi

# Run main function
main "$@"
