"""Run only against the CI disposable database seeded by seed_accounts_demo."""
import http.cookiejar
import json
import os
import urllib.error
import urllib.parse
import urllib.request

origin = os.environ.get("SMOKE_ORIGIN", "http://127.0.0.1:8187")
assert urllib.parse.urlsplit(origin).hostname == "127.0.0.1", "CI smoke test must stay on loopback"
assert os.environ.get("CI") == "true", "Use only with the CI disposable test database"
opener = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()))


def request(path, body=None, headers=None):
    request_headers = {"Origin": origin, **(headers or {})}
    if body is not None:
        request_headers["Content-Type"] = "application/json"
    req = urllib.request.Request(origin + path, data=None if body is None else json.dumps(body).encode(), headers=request_headers)
    with opener.open(req, timeout=20) as response:
        return json.load(response)


assert request("/health/ready")["storage"] == "postgresql"
login = request("/api/v1/account/login", {"email": "owner@example.test", "password": "a unique integration test passphrase"})
assert login["user"]["role"] == "owner"
key = request("/api/v1/account/keys", {"name": "CI runtime smoke key"}, {"X-CSRF-Token": login["csrf_token"]})["secret"]
puzzle = request("/api/v1/puzzles/maze?seed=42", headers={"Authorization": "Bearer " + key})
assert puzzle["kind"] == "maze" and puzzle["seed"] == "42"
assert len(request("/api/v1/catalog")["kinds"]) == 21
for kind in ["word-sorting", "sequence-detective", "memory-pairs", "path-connections", "futoshiki", "sound-families", "sentence-assembly"]:
    headers = {"Authorization": "Bearer " + key}
    generated = request(f"/api/v1/puzzles/{kind}?seed=42&include_solution=true", headers=headers)
    assert request("/api/v1/verify", {"kind": kind, "seed": 42, "difficulty": generated["difficulty"], "answer": generated["solution"]}, headers)["correct"]
    with opener.open(origin + f"/lessons/{kind}.html", timeout=10) as response:
        assert f'data-language-lab="{kind}"'.encode() in response.read()
with opener.open(origin + "/account", timeout=10) as response:
    assert b"WebAssembly" in response.read()
print("Container smoke passed: PostgreSQL readiness, login, CSRF key creation, 21-kind catalog, new generators/validators, teaching pages and frontend HTML")
