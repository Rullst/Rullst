#!/usr/bin/env python3
"""Provision a registered user in the disposable example's local database.

This is a trusted local operator command, never a public membership endpoint.
"""
import argparse
from contextlib import closing
from pathlib import Path
import re
import sqlite3


def grant_membership(database, email, tenant):
    if not re.fullmatch(r"[A-Za-z0-9_.:-]{1,128}", tenant):
        raise ValueError("Tenant must contain 1-128 letters, digits, underscores, dots, colons or hyphens")
    # mode=rw refuses to create a new database when a path was mistyped.
    uri = Path(database).resolve().as_uri() + "?mode=rw"
    with closing(sqlite3.connect(uri, uri=True, timeout=5)) as db:
        db.execute("PRAGMA foreign_keys = ON")
        with db:
            user = db.execute("SELECT id FROM users WHERE email = ?", (email.strip().lower(),)).fetchone()
            if user is None:
                raise ValueError("Register the disposable account in the example before granting membership")
            db.execute("INSERT INTO journey_memberships (user_id, tenant_id) VALUES (?, ?) "
                       "ON CONFLICT (user_id, tenant_id) DO NOTHING", (user[0], tenant))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--database", required=True, type=Path)
    parser.add_argument("--email", required=True)
    parser.add_argument("--tenant", required=True)
    args = parser.parse_args()
    grant_membership(args.database, args.email, args.tenant)
    print("Local example membership granted.")


if __name__ == "__main__":
    main()
