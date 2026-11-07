#!/usr/bin/env python3
"""Static checks for the TraceLamp product site. No network."""

from __future__ import annotations

import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent
REPO = ROOT.parent
PUBLIC = ROOT / "public"
INDEX = PUBLIC / "index.html"
NOT_FOUND = PUBLIC / "404.html"
CSS = PUBLIC / "styles.css"
WRANGLER = ROOT / "wrangler.jsonc"

REQUIRED_IDS = ("scanner", "install", "use", "mcp", "sarif", "limits", "docs")
REQUIRED_TEXT = (
    "tracelamp scan",
    "tracelamp mcp serve",
    "SARIF",
    "Apache",
    "Poietes LLC",
    "recon",
    "OPENAI_API_KEY",
    "ANTHROPIC_API_KEY",
    "OLLAMA_HOST",
    "recon/shell-execution",
    "recon/agent-cloud-spend",
    "https://github.com/cdobratz/TraceLamp/releases/tag/v0.1.0",
)
FORBIDDEN = (
    "\u2014",
    "\u2013",
    "cargo install",
    "azguard",
    "AzGuard",
    "/releases/download",
    "/releases/latest",
    "account_id",
    "CLOUDFLARE_API_TOKEN=",
)


def fail(message: str) -> None:
    print(f"site check: {message}", file=sys.stderr)
    raise SystemExit(1)


def strip_jsonc(text: str) -> str:
    lines = []
    for line in text.splitlines():
        if line.strip().startswith("//"):
            continue
        lines.append(line)
    return "\n".join(lines)


def check_html(path: pathlib.Path) -> str:
    text = path.read_text(encoding="utf-8")
    if not text.startswith("<!DOCTYPE html>"):
        fail(f"{path.name} is missing a doctype")
    void = {"meta", "link", "br", "img", "input", "hr", "source"}
    stack: list[str] = []
    for raw in re.finditer(r"<(/?)([a-zA-Z0-9]+)([^>]*)>", text):
        closing, name, rest = raw.group(1), raw.group(2).lower(), raw.group(3)
        if name in void or rest.rstrip().endswith("/"):
            if closing:
                fail(f"{path.name} closes void element {name}")
            continue
        if closing:
            if not stack or stack[-1] != name:
                fail(f"{path.name} has unbalanced </{name}>")
            stack.pop()
        else:
            stack.append(name)
    if stack:
        fail(f"{path.name} has unclosed tags: {stack}")
    if 'lang="en"' not in text:
        fail(f"{path.name} is missing lang")
    return text


def cargo_version() -> str:
    for line in (REPO / "Cargo.toml").read_text(encoding="utf-8").splitlines():
        if line.startswith("version "):
            return line.split("=", 1)[1].strip().strip('"')
    fail("Cargo.toml has no version")
    return ""


def main() -> None:
    version = cargo_version()
    index = check_html(INDEX)
    missing = check_html(NOT_FOUND)
    css = CSS.read_text(encoding="utf-8")
    wrangler_text = WRANGLER.read_text(encoding="utf-8")
    combined = "\n".join((index, missing, css, wrangler_text))
    lowered = combined.lower()
    for needle in FORBIDDEN:
        found = needle in combined or (needle.isascii() and needle.lower() in lowered)
        if found:
            fail(f"forbidden text present: {needle!r}")
    for section_id in REQUIRED_IDS:
        if f'id="{section_id}"' not in index:
            fail(f"index is missing #{section_id}")
    for phrase in REQUIRED_TEXT:
        if phrase not in index:
            fail(f"index is missing {phrase!r}")
    if version not in index:
        fail(f"index does not mention crate version {version}")
    if 'href="/styles.css"' not in index or 'href="/styles.css"' not in missing:
        fail("pages must link /styles.css so the 404 page works at any path")
    config = json.loads(strip_jsonc(wrangler_text))
    if "main" in config:
        fail("wrangler config must not set a Worker script")
    assets = config.get("assets") or {}
    if assets.get("directory") != "./public":
        fail("assets.directory must be ./public")
    if assets.get("not_found_handling") != "404-page":
        fail("not_found_handling must be 404-page")
    if "binding" in assets:
        fail("assets-only config must not set an ASSETS binding")
    patterns = {route.get("pattern"): route.get("custom_domain") for route in config.get("routes", [])}
    if patterns.get("tracelamp.dev") is not True or patterns.get("www.tracelamp.dev") is not True:
        fail("custom domains for tracelamp.dev and www.tracelamp.dev are required")
    if re.search(r"\b[0-9a-f]{32}\b", wrangler_text):
        fail("wrangler config contains a guessed account or zone id")
    print(f"site check ok ({version})")


if __name__ == "__main__":
    main()
