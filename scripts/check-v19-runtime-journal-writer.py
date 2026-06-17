#!/usr/bin/env python3
"""Executable SQLite fixture for VAC v1.9 runtime journal writer lease semantics."""
from __future__ import annotations

import sqlite3
import sys
import tempfile
from pathlib import Path
from typing import Any

ROOT = Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
RUNTIME_SQL = ROOT / ".vac/migrations/runtime-db/0001_runtime_journal.sql"
SESSION_ID = "session.writer_fixture"
WORKSPACE_ID = "workspace.fixture"
NOW = "2026-01-01T00:00:00Z"


def fail(message: str) -> None:
    print("VAC runtime journal writer lease: FAIL")
    print(f"- {message}")
    raise SystemExit(1)


def connect(db_path: Path, timeout: float = 1.0) -> sqlite3.Connection:
    conn = sqlite3.connect(db_path, timeout=timeout, isolation_level=None)
    conn.execute("PRAGMA foreign_keys = ON")
    conn.execute("PRAGMA busy_timeout = 50")
    return conn


def apply_migration(db_path: Path) -> sqlite3.Connection:
    if not RUNTIME_SQL.is_file():
        fail(f"missing runtime DB migration: {RUNTIME_SQL.relative_to(ROOT)}")
    conn = connect(db_path)
    conn.executescript(RUNTIME_SQL.read_text(encoding="utf-8"))
    conn.execute("PRAGMA foreign_keys = ON")
    journal_mode = conn.execute("PRAGMA journal_mode").fetchone()[0]
    foreign_keys = conn.execute("PRAGMA foreign_keys").fetchone()[0]
    busy_timeout = conn.execute("PRAGMA busy_timeout").fetchone()[0]
    if str(journal_mode).lower() != "wal":
        fail(f"journal_mode is {journal_mode!r}, expected wal")
    if foreign_keys != 1:
        fail("foreign_keys pragma is not enabled")
    if busy_timeout < 50:
        fail(f"busy_timeout unexpectedly low: {busy_timeout}")
    return conn


def create_session(conn: sqlite3.Connection) -> None:
    conn.execute(
        """
        INSERT INTO runtime_sessions(
          session_id, started_at, status, user_prompt_summary, current_phase,
          manifest_set_hash, compiled_snapshot_id, git_head, git_dirty_tree_hash,
          default_execution_claim, default_custody_claim
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        """,
        (
            SESSION_ID,
            NOW,
            "open",
            "writer lease fixture",
            "executing",
            "sha256:current",
            "snapshot.current",
            "0123456789abcdef0123456789abcdef01234567",
            "sha256:dirty",
            "observed_l1",
            "local_only",
        ),
    )


def recover_or_acquire_lease(
    conn: sqlite3.Connection,
    holder_id: str,
    expected_counter: int,
    timestamp: str,
    expires_at: str | None = None,
) -> int:
    conn.execute("BEGIN IMMEDIATE")
    try:
        cursor = conn.execute(
            """
            INSERT INTO runtime_writer_leases(
              workspace_id, holder_id, holder_process, lease_reason, acquired_at,
              heartbeat_at, heartbeat_counter, expires_at, session_id
            ) VALUES (?, ?, ?, ?, ?, ?, 1, ?, ?)
            ON CONFLICT(workspace_id) DO UPDATE SET
              holder_id=excluded.holder_id,
              holder_process=excluded.holder_process,
              lease_reason=excluded.lease_reason,
              heartbeat_at=excluded.heartbeat_at,
              heartbeat_counter=runtime_writer_leases.heartbeat_counter + 1,
              expires_at=excluded.expires_at,
              session_id=excluded.session_id
            WHERE runtime_writer_leases.heartbeat_counter = ?
            """,
            (
                WORKSPACE_ID,
                holder_id,
                "fixture-process",
                "writer lease fixture",
                timestamp,
                timestamp,
                expires_at,
                SESSION_ID,
                expected_counter,
            ),
        )
        rowcount = cursor.rowcount
        conn.commit()
        return rowcount
    except Exception:
        conn.rollback()
        raise


def heartbeat(conn: sqlite3.Connection, holder_id: str, timestamp: str) -> None:
    conn.execute("BEGIN IMMEDIATE")
    conn.execute(
        """
        UPDATE runtime_writer_leases
           SET heartbeat_counter = heartbeat_counter + 1,
               heartbeat_at = ?
         WHERE workspace_id = ? AND holder_id = ?
        """,
        (timestamp, WORKSPACE_ID, holder_id),
    )
    conn.commit()


def lease_row(conn: sqlite3.Connection) -> tuple[str, int, str | None]:
    row = conn.execute(
        "SELECT holder_id, heartbeat_counter, expires_at FROM runtime_writer_leases WHERE workspace_id = ?",
        (WORKSPACE_ID,),
    ).fetchone()
    if row is None:
        fail("writer lease row missing")
    return row


def assert_begin_immediate_excludes_second_writer(db_path: Path) -> None:
    conn_a = connect(db_path)
    conn_b = connect(db_path, timeout=0.05)
    try:
        conn_a.execute("BEGIN IMMEDIATE")
        try:
            conn_b.execute("BEGIN IMMEDIATE")
        except sqlite3.OperationalError as exc:
            if "locked" not in str(exc).lower():
                fail(f"second writer failed for unexpected reason: {exc}")
            return
        fail("second writer acquired BEGIN IMMEDIATE while first writer held it")
    finally:
        conn_a.rollback()
        conn_a.close()
        conn_b.close()


def allocate_sequence(conn: sqlite3.Connection, timestamp: str) -> int:
    row = conn.execute(
        """
        INSERT INTO runtime_session_sequences(session_id, next_seq, updated_at)
        VALUES (?, 2, ?)
        ON CONFLICT(session_id) DO UPDATE SET
          next_seq = runtime_session_sequences.next_seq + 1,
          updated_at = excluded.updated_at
        RETURNING next_seq - 1
        """,
        (SESSION_ID, timestamp),
    ).fetchone()
    return int(row[0])


def insert_event(conn: sqlite3.Connection, event_id: str, seq: int) -> None:
    conn.execute(
        """
        INSERT INTO runtime_events(
          event_id, session_id, seq, occurred_at, phase, event_type, severity,
          summary, manifest_set_hash, git_head, payload_cbor, content_hash,
          previous_hash, trust_claim_override_cbor, proof_ref
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        """,
        (
            event_id,
            SESSION_ID,
            seq,
            NOW,
            "executing",
            "writer_fixture_event",
            "info",
            "writer fixture event",
            "sha256:current",
            "0123456789abcdef0123456789abcdef01234567",
            None,
            f"sha256:content-{seq}",
            None,
            None,
            None,
        ),
    )


def append_event_transactionally(conn: sqlite3.Connection, event_id: str, timestamp: str) -> int:
    conn.execute("BEGIN IMMEDIATE")
    try:
        seq = allocate_sequence(conn, timestamp)
        insert_event(conn, event_id, seq)
        conn.commit()
        return seq
    except Exception:
        conn.rollback()
        raise


def assert_sequence_allocation_is_transactional(conn: sqlite3.Connection) -> None:
    seq1 = append_event_transactionally(conn, "event.writer.1", "2026-01-01T00:00:01Z")
    seq2 = append_event_transactionally(conn, "event.writer.2", "2026-01-01T00:00:02Z")
    if (seq1, seq2) != (1, 2):
        fail(f"unexpected committed seq allocation {(seq1, seq2)}")

    conn.execute("BEGIN IMMEDIATE")
    rolled_back_seq = allocate_sequence(conn, "2026-01-01T00:00:03Z")
    if rolled_back_seq != 3:
        fail(f"unexpected rolled-back candidate seq {rolled_back_seq}")
    conn.rollback()

    seq3 = append_event_transactionally(conn, "event.writer.3", "2026-01-01T00:00:04Z")
    if seq3 != 3:
        fail(f"sequence allocation was not transaction-scoped; got {seq3}, expected 3")

    try:
        insert_event(conn, "event.writer.duplicate", seq3)
    except sqlite3.IntegrityError:
        return
    fail("runtime_events duplicate (session_id, seq) insert was not rejected")


def assert_heartbeat_counter_recovery(conn: sqlite3.Connection) -> None:
    inserted = recover_or_acquire_lease(
        conn,
        holder_id="holder.a",
        expected_counter=0,
        timestamp="2026-01-01T00:00:00Z",
        expires_at="2000-01-01T00:00:00Z",
    )
    if inserted != 1:
        fail(f"initial writer lease insert affected {inserted} rows")

    holder, counter, expires_at = lease_row(conn)
    if holder != "holder.a" or counter != 1:
        fail(f"unexpected initial lease row holder={holder} counter={counter}")
    if expires_at != "2000-01-01T00:00:00Z":
        fail("expires_at fixture value was not stored")

    # A stale observer may recover only while the counter remains unchanged.
    recovered = recover_or_acquire_lease(
        conn,
        holder_id="holder.b",
        expected_counter=1,
        timestamp="2026-01-01T00:00:10Z",
    )
    if recovered != 1:
        fail("stale recovery with unchanged heartbeat_counter did not succeed")
    holder, counter, _ = lease_row(conn)
    if holder != "holder.b" or counter != 2:
        fail(f"unexpected recovered lease holder={holder} counter={counter}")

    # Wall-clock expiry alone is not authority: a stale expected counter cannot recover.
    heartbeat(conn, "holder.b", "2026-01-01T00:00:20Z")
    rejected = recover_or_acquire_lease(
        conn,
        holder_id="holder.c",
        expected_counter=1,
        timestamp="2026-01-01T00:00:30Z",
        expires_at="2000-01-01T00:00:00Z",
    )
    holder, counter, _ = lease_row(conn)
    if rejected != 0 or holder != "holder.b" or counter != 3:
        fail(
            "stale recovery used wall-clock expiry or stale counter incorrectly: "
            f"rowcount={rejected} holder={holder} counter={counter}"
        )


def assert_deferred_stale_recovery_rejected(db_path: Path) -> None:
    conn_a = connect(db_path)
    conn_b = connect(db_path)
    try:
        conn_a.execute("BEGIN DEFERRED")
        observed = conn_a.execute(
            "SELECT heartbeat_counter FROM runtime_writer_leases WHERE workspace_id = ?",
            (WORKSPACE_ID,),
        ).fetchone()[0]
        heartbeat(conn_b, "holder.b", "2026-01-01T00:00:40Z")
        try:
            conn_a.execute(
                """
                UPDATE runtime_writer_leases
                   SET holder_id = ?, heartbeat_counter = heartbeat_counter + 1
                 WHERE workspace_id = ? AND heartbeat_counter = ?
                """,
                ("holder.deferred", WORKSPACE_ID, observed),
            )
            if conn_a.total_changes != 0:
                fail("DEFERRED stale recovery unexpectedly updated the writer lease")
        except sqlite3.OperationalError as exc:
            if "locked" not in str(exc).lower():
                fail(f"DEFERRED stale recovery failed for unexpected reason: {exc}")
        finally:
            conn_a.rollback()
    finally:
        conn_a.close()
        conn_b.close()


def main() -> int:
    with tempfile.TemporaryDirectory(prefix="vac-runtime-journal-writer-") as tmp:
        db_path = Path(tmp) / "runtime.db"
        conn = apply_migration(db_path)
        try:
            create_session(conn)
            assert_begin_immediate_excludes_second_writer(db_path)
            assert_heartbeat_counter_recovery(conn)
            assert_sequence_allocation_is_transactional(conn)
            assert_deferred_stale_recovery_rejected(db_path)
        finally:
            conn.close()

    print("VAC runtime journal writer lease: PASS")
    print("begin_immediate_writer_lock=TV-Pass")
    print("heartbeat_counter_recovery=TV-Pass")
    print("expires_at_not_authority=true")
    print("sequence_allocation_transactional=true")
    print("duplicate_session_seq_rejected=true")
    print("deferred_stale_recovery_rejected=true")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
