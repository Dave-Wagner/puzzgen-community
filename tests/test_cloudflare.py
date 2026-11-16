"""No real Cloudflare calls or credentials. Run python3 -m unittest discover -s tests -p 'test_*.py'."""
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location("cloudflare", Path(__file__).resolve().parents[1] / "deploy/cloudflare.py")
cloudflare = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cloudflare)


class CloudflareTests(unittest.TestCase):
    def invoke(self, records, dry_run=False):
        calls = []

        def fake(token, method, path, payload=None):
            self.assertEqual(token, "fake-test-token")
            calls.append((method, path, payload))
            if path.startswith("/zones?"):
                return [{"id": "zone-id", "name": "davewagner.org"}]
            if method == "GET":
                return records
            return {"id": "record-id"}

        result = cloudflare.configure("fake-test-token", "13.140.147.253", dry_run=dry_run, call=fake)
        return result, calls

    def test_create_only_requested_record(self):
        result, calls = self.invoke([])
        self.assertEqual(result, "created")
        self.assertEqual(calls[-1][0:2], ("POST", "/zones/zone-id/dns_records"))
        self.assertEqual(calls[-1][2]["name"], "puzzlegen.davewagner.org")
        self.assertTrue(calls[-1][2]["proxied"])

    def test_update_existing_record(self):
        result, calls = self.invoke([{"type": "A", "id": "old", "content": "1.1.1.1"}])
        self.assertEqual(result, "updated")
        self.assertEqual(calls[-1][0:2], ("PUT", "/zones/zone-id/dns_records/old"))

    def test_idempotent(self):
        result, calls = self.invoke([{"id": "old", "type": "A", "name": "puzzlegen.davewagner.org", "content": "13.140.147.253", "ttl": 1, "proxied": True}])
        self.assertEqual(result, "unchanged")
        self.assertEqual(len(calls), 2)

    def test_dry_run_does_not_mutate(self):
        result, calls = self.invoke([], dry_run=True)
        self.assertEqual(result, "would create")
        self.assertTrue(all(call[0] == "GET" for call in calls))

    def test_refuse_conflicting_records(self):
        for records in [[{"type": "CNAME"}], [{"type": "AAAA"}], [{"type": "A"}, {"type": "A"}]]:
            with self.assertRaises(ValueError):
                self.invoke(records)

    def test_refuse_private_origin_or_outside_zone(self):
        for kwargs in [{"ipv4": "127.0.0.1"}, {"ipv4": "13.140.147.253", "hostname": "other.example"}]:
            with self.assertRaises(ValueError):
                cloudflare.configure("fake-test-token", **kwargs)


if __name__ == "__main__":
    unittest.main()
