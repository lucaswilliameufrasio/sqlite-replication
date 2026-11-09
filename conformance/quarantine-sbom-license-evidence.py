#!/usr/bin/env python3
"""Quarantine two misleading detector outputs without asserting a license."""

import argparse
import json
from pathlib import Path


EXPECTED = {
    ("modernc.org/libc", "v1.80.1"): "MIT",
    ("modernc.org/sqlite", "v1.61.1"): "Apache-2.0",
}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path)
    args = parser.parse_args()
    data = json.loads(args.source.read_text(encoding="utf-8"))
    if data.get("bomFormat") != "CycloneDX" or data.get("specVersion") != "1.6":
        raise ValueError("expected CycloneDX 1.6 JSON")
    seen = set()
    for component in data.get("components", []):
        key = (component.get("name"), component.get("version"))
        if key not in EXPECTED:
            continue
        if key in seen:
            raise ValueError(f"duplicate component: {key}")
        evidence = component.get("evidence", {})
        expected_evidence = [{"license": {"id": EXPECTED[key]}}]
        if evidence.get("licenses") != expected_evidence:
            raise ValueError(f"unexpected evidence for {key}")
        del evidence["licenses"]
        if not evidence:
            del component["evidence"]
        seen.add(key)
    if seen != set(EXPECTED):
        raise ValueError(f"missing expected components: {set(EXPECTED) - seen}")
    with args.destination.open("x", encoding="utf-8") as output:
        json.dump(data, output, indent=2, ensure_ascii=False)
        output.write("\n")


if __name__ == "__main__":
    main()
