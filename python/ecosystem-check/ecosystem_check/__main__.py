"""
Enables usage with `python -m ecosystem_check`
"""

from __future__ import annotations

import ecosystem_check.cli

if __name__ == "__main__":
    ecosystem_check.cli.entrypoint()
