"""Stand-in for the Hermes TUI gateway, used only by sayn-hermes-runtime tests.

Speaks newline-delimited JSON-RPC 2.0 on stdio. Method names, parameter keys
and the unknown-key refusal (error 4000) follow the wire contracts of the
vendored upstream revision (tui_gateway/contracts/); only the keys the Sayn
client uses are modelled. The first argument selects a behaviour to exercise
one client code path.
"""

import json
import sys
import time

mode = sys.argv[1] if len(sys.argv) > 1 else "normal"


def send(message):
    sys.stdout.write(json.dumps(message) + "\n")
    sys.stdout.flush()


def event(kind, payload=None):
    send({"jsonrpc": "2.0", "method": "event", "params": {"type": kind, "payload": payload or {}}})


def result(message_id, value):
    send({"jsonrpc": "2.0", "id": message_id, "result": value})


def error(message_id, code, text):
    send({"jsonrpc": "2.0", "id": message_id, "error": {"code": code, "message": text}})


# Parameter keys the real contracts accept, for the methods Sayn calls.
ALLOWED = {
    "client.capabilities": {"server_requests", "desktop_local"},
    "session.create": {"cwd", "cwd_explicit", "model", "provider", "title", "close_on_disconnect"},
    "prompt.submit": {"session_id", "text"},
    "session.interrupt": {"session_id"},
}


if mode == "no_ready":
    time.sleep(60)
    sys.exit(0)

if mode == "noisy_start":
    sys.stdout.write("loading plugins...\n")
    sys.stdout.flush()

event("gateway.ready", {"skin": {}, "change_events": True, "replay_epoch": 1})

if mode == "eof_after_ready":
    sys.exit(0)

while True:
    line = sys.stdin.readline()
    if not line:
        break
    if not line.strip():
        continue
    message = json.loads(line)
    method = message.get("method")
    message_id = message.get("id")
    params = message.get("params") or {}

    unknown = set(params) - ALLOWED.get(method, set(params))
    if unknown:
        error(message_id, 4000, "unknown parameter: " + sorted(unknown)[0])
        continue

    if method == "client.capabilities":
        result(message_id, {"server_requests": ["approval", "clarify", "sudo"], "declines_not_shown": True})
    elif method == "session.interrupt":
        result(message_id, {"status": "not_interrupted"})
    elif method == "session.create":
        if mode == "hang":
            time.sleep(60)
        if mode == "eof_mid_call":
            sys.exit(0)
        result(message_id, {"session_id": "sess-1", "stored_session_id": "stored-1", "message_count": 0})
    elif method == "prompt.submit":
        if mode == "interleaved":
            event("token", {"text": "hel"})
            result(9999, {"stale": True})
            event("token", {"text": "lo"})
        if mode == "server_request":
            send(
                {
                    "jsonrpc": "2.0",
                    "id": "srq-0123456789ab",
                    "method": "approval",
                    "params": {"command": "rm -rf /"},
                }
            )
            reply = json.loads(sys.stdin.readline())
            result(message_id, {"accepted": True, "server_request_reply": reply})
            continue
        result(message_id, {"accepted": True})
        event("turn.complete", {"session_id": params.get("session_id")})
    elif method == "explode":
        error(message_id, -32000, "boom")
    else:
        error(message_id, -32601, "method not found")
