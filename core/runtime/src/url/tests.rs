use crate::test::{TestAction, run_test_actions};

const TEST_HARNESS: &str = r#"
function assert(condition, message) {
    if (!condition) {
        if (!message) {
            message = "Assertion failed";
        }
        throw new Error(message);
    }
}

function assert_eq(a, b, message) {
    if (a !== b) {
        throw new Error(`${message} (${JSON.stringify(a)} !== ${JSON.stringify(b)})`);
    }
}
"#;

#[test]
fn url_basic() {
    run_test_actions([
        TestAction::run(TEST_HARNESS),
        TestAction::run(
            r##"
                url = new URL("https://example.com:8080/path/to/resource?query#fragment");
                assert(url instanceof URL);
                assert_eq(url.href, "https://example.com:8080/path/to/resource?query#fragment");
                assert_eq(url.protocol, "https:");
                assert_eq(url.host, "example.com:8080");
                assert_eq(url.hostname, "example.com");
                assert_eq(url.port, "8080");
                assert_eq(url.pathname, "/path/to/resource");
                assert_eq(url.search, "?query");
                assert_eq(url.hash, "#fragment");
            "##,
        ),
    ]);
}

#[test]
fn url_base() {
    run_test_actions([
        TestAction::run(TEST_HARNESS),
        TestAction::run(
            r##"
                url = new URL("https://example.com:8080/path/to/resource?query#fragment", "http://example.org/");
                assert_eq(url.href, "https://example.com:8080/path/to/resource?query#fragment");
                assert_eq(url.protocol, "https:");
                assert_eq(url.host, "example.com:8080");
                assert_eq(url.hostname, "example.com");
                assert_eq(url.port, "8080");
                assert_eq(url.pathname, "/path/to/resource");
                assert_eq(url.search, "?query");
                assert_eq(url.hash, "#fragment");
            "##,
        ),
        TestAction::run(
            r##"
                url = new URL("/path/to/resource?query#fragment", "http://example.org/");
                assert_eq(url.href, "http://example.org/path/to/resource?query#fragment");
                assert_eq(url.protocol, "http:");
                assert_eq(url.host, "example.org");
                assert_eq(url.hostname, "example.org");
                assert_eq(url.port, "");
                assert_eq(url.pathname, "/path/to/resource");
                assert_eq(url.search, "?query");
                assert_eq(url.hash, "#fragment");
            "##,
        ),
    ]);
}

#[test]
fn url_setters() {
    // These were double checked against Firefox.
    run_test_actions([
        TestAction::run(TEST_HARNESS),
        TestAction::run(
            r##"
                url = new URL("https://example.com:8080/path/to/resource?query#fragment");
                url.protocol = "http:";
                url.host = "example.org:80"; // Since protocol is http, port is removed.
                url.pathname = "/new/path";
                url.search = "?new-query";
                url.hash = "#new-fragment";
                assert_eq(url.href, "http://example.org/new/path?new-query#new-fragment");
                assert_eq(url.protocol, "http:");
                assert_eq(url.host, "example.org");
                assert_eq(url.hostname, "example.org");
                assert_eq(url.port, "");
                assert_eq(url.pathname, "/new/path");
                assert_eq(url.search, "?new-query");
                assert_eq(url.hash, "#new-fragment");
            "##,
        ),
    ]);
}

#[test]
fn url_search_params_standalone() {
    run_test_actions([
        TestAction::run(TEST_HARNESS),
        TestAction::run(
            r##"
                // Construct from string
                p = new URLSearchParams("foo=1&bar=2&foo=3");
                assert(p instanceof URLSearchParams);
                assert_eq(p.size, 3, "size");
                assert_eq(p.get("foo"), "1", "get first");
                assert_eq(p.get("bar"), "2", "get bar");
                assert_eq(p.get("missing"), null, "get missing");
                assert(p.has("foo"), "has foo");
                assert(!p.has("nope"), "has nope");
                assert_eq(JSON.stringify(p.getAll("foo")), '["1","3"]', "getAll");
            "##,
        ),
        TestAction::run(
            r##"
                // Leading ? is stripped
                p = new URLSearchParams("?a=x&b=y");
                assert_eq(p.get("a"), "x", "leading ? stripped");
                assert_eq(p.get("b"), "y", "leading ? b");
            "##,
        ),
        TestAction::run(
            r##"
                // Construct from array of pairs
                p = new URLSearchParams([["k1","v1"],["k2","v2"]]);
                assert_eq(p.get("k1"), "v1", "array k1");
                assert_eq(p.get("k2"), "v2", "array k2");
            "##,
        ),
        TestAction::run(
            r##"
                // Construct from record
                p = new URLSearchParams({x: "10", y: "20"});
                assert_eq(p.get("x"), "10", "record x");
                assert_eq(p.get("y"), "20", "record y");
            "##,
        ),
        TestAction::run(
            r##"
                // append / delete / set
                p = new URLSearchParams("a=1&b=2&a=3");
                p.append("c", "4");
                assert_eq(p.size, 4, "after append");
                p.delete("a");
                assert_eq(p.size, 2, "after delete all a");
                p.set("b", "99");
                assert_eq(p.get("b"), "99", "after set");
                p.set("d", "5");
                assert_eq(p.get("d"), "5", "set new key");
            "##,
        ),
        TestAction::run(
            r##"
                // sort
                p = new URLSearchParams("b=2&a=1&c=3");
                p.sort();
                assert_eq(JSON.stringify(p.keys()), '["a","b","c"]', "sorted keys");
            "##,
        ),
        TestAction::run(
            r##"
                // toString
                p = new URLSearchParams("foo=bar&baz=qux");
                assert_eq(p.toString(), "foo=bar&baz=qux", "toString");
            "##,
        ),
        TestAction::run(
            r##"
                // keys / values / entries
                p = new URLSearchParams("x=1&y=2");
                assert_eq(JSON.stringify(p.keys()), '["x","y"]', "keys");
                assert_eq(JSON.stringify(p.values()), '["1","2"]', "values");
                e = p.entries();
                assert_eq(e[0][0], "x", "entry 0 key");
                assert_eq(e[0][1], "1", "entry 0 val");
            "##,
        ),
        TestAction::run(
            r##"
                // has with value
                p = new URLSearchParams("a=1&a=2");
                assert(p.has("a"), "has a");
                assert(p.has("a", "1"), "has a=1");
                assert(!p.has("a", "3"), "no a=3");
                // delete with value
                p.delete("a", "1");
                assert_eq(p.size, 1, "after delete a=1");
                assert_eq(p.get("a"), "2", "remaining a=2");
            "##,
        ),
        TestAction::run(
            r##"
                // Copy from another URLSearchParams
                p1 = new URLSearchParams("x=1");
                p2 = new URLSearchParams(p1);
                p2.set("x", "2");
                assert_eq(p1.get("x"), "1", "original unchanged");
                assert_eq(p2.get("x"), "2", "copy updated");
            "##,
        ),
        TestAction::run(
            r##"
                // forEach
                p = new URLSearchParams("a=1&b=2");
                collected = [];
                p.forEach((val, key) => collected.push(key + "=" + val));
                assert_eq(JSON.stringify(collected), '["a=1","b=2"]', "forEach");
            "##,
        ),
    ]);
}

#[test]
fn url_search_params_live_link() {
    run_test_actions([
        TestAction::run(TEST_HARNESS),
        TestAction::run(
            r##"
                // searchParams from URL
                url = new URL("https://example.com/path?foo=bar&baz=qux");
                p = url.searchParams;
                assert(p instanceof URLSearchParams, "instanceof");
                assert_eq(p.get("foo"), "bar", "get foo");
                assert_eq(p.get("baz"), "qux", "get baz");
            "##,
        ),
        TestAction::run(
            r##"
                // Mutating searchParams updates URL.search
                url = new URL("https://example.com/?a=1");
                p = url.searchParams;
                p.set("a", "2");
                assert_eq(url.search, "?a=2", "search updated after set");
                p.append("b", "3");
                assert(url.search.includes("b=3"), "search includes b=3");
            "##,
        ),
        TestAction::run(
            r##"
                // Deleting all params clears URL.search
                url = new URL("https://example.com/?x=1");
                p = url.searchParams;
                p.delete("x");
                assert_eq(url.search, "", "search empty after delete");
            "##,
        ),
        TestAction::run(
            r##"
                // URL with no query
                url = new URL("https://example.com/");
                p = url.searchParams;
                assert_eq(p.size, 0, "empty searchParams");
                p.append("k", "v");
                assert_eq(url.search, "?k=v", "search set after append");
            "##,
        ),
    ]);
}

#[test]
fn url_static_methods() {
    run_test_actions([
        TestAction::run(TEST_HARNESS),
        TestAction::run(
            r##"
                assert(URL.canParse("http://example.org/new/path?new-query#new-fragment"));
                assert(!URL.canParse("http//:example.org/new/path?new-query#new-fragment"));
                assert(!URL.canParse("http://example.org/new/path?new-query#new-fragment", "http:"));
                assert(URL.canParse("/new/path?new-query#new-fragment", "http://example.org/"));
            "##,
        ),
    ]);
}
