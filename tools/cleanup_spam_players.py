#!/usr/bin/env python3
"""Remove spam/test players from game_data/state.json and neutralize their nodes."""
import json
import shutil
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
STATE_PATH = ROOT / "game_data" / "state.json"
BACKUP_PATH = ROOT / "game_data" / "state.json.backup"

# Explicit test accounts created during development.
EXPLICIT_TEST_USERS = {
    "colortester",
    "colorpicker2",
    "colorconf2",
    "colorconf3",
    "colorconf4",
    "colorconf5",
}


def is_spam_username(username: str) -> bool:
    lower = username.lower()
    if any(k in lower for k in ("test", "probe", "perf", "dummy")):
        return True
    if lower.startswith("qa_") or "loginprobe" in lower or "trae_" in lower:
        return True
    if lower.isdigit():
        return True
    if len(username) >= 12 and len(set(lower)) <= 4:
        return True
    if username in EXPLICIT_TEST_USERS:
        return True
    return False


def main():
    if not STATE_PATH.exists():
        print(f"State file not found: {STATE_PATH}", file=sys.stderr)
        sys.exit(1)

    print(f"Loading {STATE_PATH} ...")
    with open(STATE_PATH, "r", encoding="utf-8") as f:
        state = json.load(f)

    players = state.get("players", {})
    usernames = state.get("usernames", {})
    sessions = state.get("sessions", {})
    nodes = state.get("nodes", {})
    attacks = state.get("attacks", {})

    spam_ids = {pid for pid, p in players.items() if is_spam_username(p.get("username", ""))}
    print(f"Found {len(spam_ids)} spam/test players out of {len(players)} total.")
    if not spam_ids:
        print("Nothing to do.")
        return

    for pid in sorted(spam_ids):
        p = players[pid]
        print(f"  - {p.get('username')} ({pid})")

    # Neutralize nodes owned by spam players.
    nodes_reset = 0
    for node_id, node in nodes.items():
        if node.get("ownerId") in spam_ids:
            node["ownerId"] = None
            node["army"] = 10
            nodes_reset += 1
    print(f"Reset {nodes_reset} nodes to neutral.")

    # Remove attacks owned by spam players.
    attacks_removed = 0
    new_attacks = {}
    for aid, attack in attacks.items():
        if attack.get("ownerId") in spam_ids:
            attacks_removed += 1
        else:
            new_attacks[aid] = attack
    state["attacks"] = new_attacks
    print(f"Removed {attacks_removed} attacks.")

    # Remove spam players from players and usernames maps.
    for pid in spam_ids:
        players.pop(pid, None)

    new_usernames = {u: pid for u, pid in usernames.items() if pid not in spam_ids}
    state["usernames"] = new_usernames

    # Remove sessions for spam players.
    new_sessions = {token: s for token, s in sessions.items() if s.get("playerId") not in spam_ids}
    removed_sessions = len(sessions) - len(new_sessions)
    state["sessions"] = new_sessions
    print(f"Removed {removed_sessions} sessions.")

    print(f"Remaining players: {len(state['players'])}")

    # Backup and write.
    print(f"Backing up original state to {BACKUP_PATH}")
    shutil.copy2(STATE_PATH, BACKUP_PATH)

    print(f"Writing cleaned state to {STATE_PATH}")
    with open(STATE_PATH, "w", encoding="utf-8") as f:
        json.dump(state, f)

    print("Done.")


if __name__ == "__main__":
    main()
