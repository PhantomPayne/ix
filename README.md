# ix

`ix` is an interactive index-and-pick tool for the shell. It lists items from
various sources — git status, files, processes, branches, docker containers —
assigns each a numbered slot, then lets you reference items by slot number
directly on the command line.

```
$ ix gs
[1]  ??  README.md
[2]  M   src/main.rs
[3]  A   src/lib.rs

$ git add $(ix 1 2)
# expands to: git add /home/user/project/README.md /home/user/project/src/main.rs
```

---

## Installation

Build from source with Cargo:

```zsh
cargo install --path crates/ix-cli
```

---

## Zsh Setup

Add the shell integration to your `~/.zshrc`:

```zsh
eval "$(ix --shell-init)"
```

This binds **Ctrl+X** to an interactive picker. When you press Ctrl+X, `ix`
opens a fuzzy-search UI over the last index. Select items with Space or by
typing slot numbers, press Enter, and the selected values are inserted at your
cursor position.

Reload your shell after editing:

```zsh
source ~/.zshrc
```

---

## Core Workflow

### 1. Build an index

Run a provider subcommand to list items and cache them:

```zsh
ix gs       # git status (default in a git repo)
ix gb       # git branches
ix gst      # git stash
ix ls       # files in current directory
ix ps       # running processes
ix dk       # docker containers
ix          # auto-detect: gs inside a git repo, ls otherwise
```

The index is stored in `.git/ix-index` inside a git repo, or
`~/.cache/ix/<hash>/index.json` otherwise.

### 2. Reference items by slot number

After building an index, pass slot numbers as arguments. `ix` prints the
corresponding raw values (full paths, PIDs, branch names) separated by spaces:

```zsh
ix 1          # single item
ix 1-3        # range: items 1, 2, 3
ix 1,3        # comma list: items 1 and 3
ix 1,3-5      # mixed: items 1, 3, 4, 5
ix 2 4        # multiple arguments: items 2 and 4
```

The output is meant to be captured with `$()`:

```zsh
git add $(ix 1-3)
kill $(ix 2)
docker exec -it $(ix 1) bash
git checkout $(ix 1)
```

### 3. Interactive picker (Ctrl+X)

Press **Ctrl+X** at any point to open the picker over the last index:

| Key | Action |
|-----|--------|
| `↑` / `k` | Move cursor up |
| `↓` / `j` | Move cursor down |
| `Space` | Toggle selection |
| `Enter` | Confirm and insert into command line |
| `Esc` / `Ctrl+C` | Cancel |
| Any character | Filter by fuzzy search or type slot numbers |

You can also invoke the picker directly:

```zsh
ix --pick
```

---

## Providers

### `gs` — Git status

Lists changed files in the working tree, grouped by stage:

```zsh
ix gs
# [1]  A   src/new.rs       ← staged add
# [2]  M   src/main.rs      ← unstaged modification
# [3]  ??  scratch.txt      ← untracked file
```

The raw value is the **full absolute path** to each file.

**Flags:**

| Flag | Description |
|------|-------------|
| `--ignored` | Also show gitignored files (`!!`) |

**Examples:**

```zsh
ix gs
git add $(ix 1-3)
git diff $(ix 2)
git restore $(ix 1)
```

---

### `gb` — Git branches

Lists local and remote branches:

```zsh
ix gb
# [1]  main          ← current HEAD marked with *
# [2]  feature/auth
# [3]  origin/main
```

The raw value is the **branch name** string.

**Examples:**

```zsh
ix gb
git checkout $(ix 2)
git merge $(ix 3)
git branch -d $(ix 2)
```

---

### `gst` — Git stash

Lists stash entries:

```zsh
ix gst
# [1]  stash@{0}  WIP on main: fix login bug
# [2]  stash@{1}  WIP on feature: partial work
```

The raw value is the **stash reference** (e.g. `stash@{0}`).

**Examples:**

```zsh
ix gst
git stash pop $(ix 1)
git stash apply $(ix 2)
git stash drop $(ix 1)
```

---

### `ls` — Files

Lists files and directories in the current directory, directories first:

```zsh
ix ls
# [1]  src/
# [2]  tests/
# [3]  Cargo.toml
# [4]  README.md
```

The raw value is the **full absolute path**.

**Flags:**

| Flag | Description |
|------|-------------|
| `-a` / `--hidden` | Show hidden files (dotfiles) |
| `-A` / `--all` | Show hidden files and gitignored files |

**Examples:**

```zsh
ix ls
cat $(ix 3)
cp $(ix 1) $(ix 2)
ix ls -a       # include .env, .gitignore, etc.
ix ls -A       # include everything
```

---

### `ps` — Processes

Lists running processes for the current user:

```zsh
ix ps
# [1]  1234  zsh
# [2]  5678  vim src/main.rs
# [3]  9012  sleep 30
```

The raw value is the **PID** as a string.

**Flags:**

| Flag | Description |
|------|-------------|
| `-a` / `--all` | Show all users' processes |

**Examples:**

```zsh
ix ps
kill $(ix 3)
lsof -p $(ix 2)
strace -p $(ix 1)
ix ps -a       # all users
```

---

### `dk` — Docker containers

Lists running Docker containers:

```zsh
ix dk
# [1]  web-server   (nginx:latest)     running
# [2]  db           (postgres:15)      running
```

The raw value is the **container name**.

**Flags:**

| Flag | Description |
|------|-------------|
| `-a` / `--all` | Also show stopped containers |

**Examples:**

```zsh
ix dk
docker exec -it $(ix 1) bash
docker logs $(ix 2)
docker stop $(ix 1)
docker rm $(ix 1)
ix dk -a       # include stopped containers
```

---

## Staleness

The index is considered stale after **5 minutes**. `ix` will print a warning to
stderr when resolving against a stale index:

```
warning: index is 7m old — run `ix` to refresh
```

To check programmatically (exits 1 if stale):

```zsh
ix --stale || ix gs
```

Useful in a prompt or pre-command hook to auto-refresh:

```zsh
# ~/.zshrc — refresh index before each command if stale
precmd() {
    ix --stale 2>/dev/null || ix 2>/dev/null
}
```

---

## Shell aliases

The git provider subcommands are short enough to use directly, but you can also
add aliases:

```zsh
# ~/.zshrc
alias gs='ix gs'
alias gb='ix gb'
alias gst='ix gst'
```

---

## Reference

| Command | Description |
|---------|-------------|
| `ix` | Auto-detect provider and build index |
| `ix gs` | Index git status |
| `ix gb` | Index git branches |
| `ix gst` | Index git stash |
| `ix ls` | Index current directory |
| `ix ps` | Index running processes |
| `ix dk` | Index docker containers |
| `ix <n>` | Resolve slot `n` to its raw value |
| `ix <n>-<m>` | Resolve a range of slots |
| `ix <n>,<m>` | Resolve a comma-separated list of slots |
| `ix --pick` | Open interactive picker |
| `ix --stale` | Exit 1 if index is older than 5 minutes |
| `ix --shell-init` | Print shell integration snippet |
