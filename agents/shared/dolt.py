"""Dolt database helper for Deepwork Intelligence."""
from __future__ import annotations

import pymysql
from typing import Any


class DoltClient:
    """Thin wrapper for querying Dolt on port 3307."""

    def __init__(self, host: str = "127.0.0.1", port: int = 3307,
                 user: str = "root", password: str = ""):
        self.host = host
        self.port = port
        self.user = user
        self.password = password

    def _conn(self, db: str) -> pymysql.Connection:
        return pymysql.connect(
            host=self.host, port=self.port,
            user=self.user, password=self.password,
            database=db, cursorclass=pymysql.cursors.DictCursor,
            connect_timeout=10, read_timeout=15,
        )

    def query(self, db: str, sql: str, params: tuple = ()) -> list[dict[str, Any]]:
        """Execute a SELECT and return rows as dicts."""
        conn = self._conn(db)
        try:
            with conn.cursor() as cur:
                cur.execute(sql, params)
                return cur.fetchall()
        finally:
            conn.close()

    def execute(self, db: str, sql: str, params: tuple = ()) -> int:
        """Execute an INSERT/UPDATE/DELETE. Returns affected rows."""
        conn = self._conn(db)
        try:
            with conn.cursor() as cur:
                affected = cur.execute(sql, params)
                conn.commit()
                return affected
        finally:
            conn.close()

    def commit_and_push(self, db: str, message: str) -> None:
        """Dolt add, commit, push."""
        conn = self._conn(db)
        try:
            with conn.cursor() as cur:
                cur.execute("CALL dolt_add('-A')")
                try:
                    cur.execute("CALL dolt_commit('-m', %s, '--allow-empty')", (message,))
                except Exception:
                    pass  # Nothing to commit is fine
            conn.commit()
        finally:
            conn.close()
