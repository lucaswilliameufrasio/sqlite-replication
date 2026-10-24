#!/usr/bin/env python3
"""Minimal stdlib-only, SigV4-signed S3 persistence probe for local Floci."""

import hashlib
import hmac
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from datetime import datetime, timezone

ACCESS_KEY = "test"
SECRET_KEY = "test"
REGION = "us-east-1"
SERVICE = "s3"
PAYLOAD_PREFIX = b"sqlite-replication Floci persistent smoke test: "


def main(arguments):
    if len(arguments) != 4:
        raise ValueError("usage: floci_smoke.py <wait|write|verify|cleanup> ENDPOINT RUN_ID")

    command, endpoint, run_id = arguments[1:]
    validate_local_endpoint(endpoint)

    if command == "wait":
        wait_until_ready(endpoint)
        return

    bucket = "sqlite-replication-smoke-" + run_id
    object_key = "persisted-object"
    payload = PAYLOAD_PREFIX + run_id.encode("ascii") + b"\n"

    if command == "write":
        request(endpoint, "PUT", "/" + bucket, b"")
        request(endpoint, "PUT", "/" + bucket + "/" + object_key, payload)
        print("Created local S3 bucket and persistence marker.")
        return

    if command == "verify":
        received = request(endpoint, "GET", "/" + bucket + "/" + object_key)
        if received != payload:
            raise RuntimeError("Floci returned different bytes after persistent restart")
        print("S3 object survived Floci stop/start with its persistent volume.")
        return

    if command == "cleanup":
        request(endpoint, "DELETE", "/" + bucket + "/" + object_key)
        request(endpoint, "DELETE", "/" + bucket)
        print("Removed the smoke-test S3 object and bucket.")
        return

    raise ValueError("unknown smoke-test command: " + command)


def validate_local_endpoint(endpoint):
    parsed = urllib.parse.urlsplit(endpoint)
    if parsed.scheme != "http" or parsed.hostname not in ("127.0.0.1", "localhost"):
        raise ValueError("Floci smoke tests may only call a local HTTP endpoint")
    if parsed.path or parsed.query or parsed.fragment:
        raise ValueError("endpoint must not include a path, query, or fragment")


def wait_until_ready(endpoint):
    deadline = time.monotonic() + 90
    paths = ("/_floci/health", "/_localstack/health")
    last_error = "no health endpoint answered"

    while time.monotonic() < deadline:
        for path in paths:
            try:
                body = unsigned_get(endpoint + path)
                if body:
                    print("Floci health endpoint is ready: " + path)
                    return
                last_error = "health endpoint returned an empty body"
            except (urllib.error.URLError, OSError) as error:
                last_error = type(error).__name__

        time.sleep(1)

    raise TimeoutError("Floci did not become ready within 90 seconds: " + last_error)


def unsigned_get(url):
    request_object = urllib.request.Request(url, method="GET")
    with urllib.request.urlopen(request_object, timeout=2) as response:
        if response.status != 200:
            raise RuntimeError("Floci health endpoint returned HTTP " + str(response.status))
        return response.read()


def request(endpoint, method, path, body=b""):
    now = datetime.now(timezone.utc)
    amz_date = now.strftime("%Y%m%dT%H%M%SZ")
    date_stamp = now.strftime("%Y%m%d")
    payload_hash = hashlib.sha256(body).hexdigest()
    parsed_endpoint = urllib.parse.urlsplit(endpoint)
    host = parsed_endpoint.netloc.lower()
    canonical_uri = urllib.parse.quote(path, safe="/-_.~")
    canonical_headers = (
        "host:" + host + "\n"
        "x-amz-content-sha256:" + payload_hash + "\n"
        "x-amz-date:" + amz_date + "\n"
    )
    signed_headers = "host;x-amz-content-sha256;x-amz-date"
    canonical_request = "\n".join(
        (method, canonical_uri, "", canonical_headers, signed_headers, payload_hash)
    )
    scope = date_stamp + "/" + REGION + "/" + SERVICE + "/aws4_request"
    string_to_sign = "\n".join(
        (
            "AWS4-HMAC-SHA256",
            amz_date,
            scope,
            hashlib.sha256(canonical_request.encode("utf-8")).hexdigest(),
        )
    )
    signing_key = hmac_sha256(
        hmac_sha256(
            hmac_sha256(
                hmac_sha256(("AWS4" + SECRET_KEY).encode("utf-8"), date_stamp),
                REGION,
            ),
            SERVICE,
        ),
        "aws4_request",
    )
    signature = hmac.new(signing_key, string_to_sign.encode("utf-8"), hashlib.sha256).hexdigest()
    authorization = (
        "AWS4-HMAC-SHA256 Credential="
        + ACCESS_KEY
        + "/"
        + scope
        + ", SignedHeaders="
        + signed_headers
        + ", Signature="
        + signature
    )
    request_object = urllib.request.Request(
        endpoint + canonical_uri,
        data=body if method in ("PUT", "POST") else None,
        method=method,
        headers={
            "Authorization": authorization,
            "x-amz-content-sha256": payload_hash,
            "x-amz-date": amz_date,
        },
    )

    try:
        with urllib.request.urlopen(request_object, timeout=10) as response:
            return response.read()
    except urllib.error.HTTPError as error:
        raise RuntimeError("local S3 request failed with HTTP " + str(error.code)) from error


def hmac_sha256(key, value):
    if isinstance(value, str):
        value = value.encode("utf-8")
    return hmac.new(key, value, hashlib.sha256).digest()


if __name__ == "__main__":
    try:
        main(sys.argv)
    except Exception as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
