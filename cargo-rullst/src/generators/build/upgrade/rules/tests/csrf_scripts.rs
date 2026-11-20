//! `V13-CSRF-TOKEN-COMPARED` in JavaScript, TypeScript and Python scripts,
//! such as browser and smoke tests, which `cargo test` does not run.

use super::super::files::{csrf_compared_lines, csrf_comparisons};
use super::files::Project;

fn js(source: &str) -> Vec<usize> {
    csrf_compared_lines(source, false)
}

fn py(source: &str) -> Vec<usize> {
    csrf_compared_lines(source, true)
}

/// The sign-in check of a Playwright script written before 13.0.0-alpha.6,
/// and the masked-token check that replaces it.
const BROWSER_SIGN_IN: &str = r#"// Signs in through the form (CSRF token).
const token = await this.page.locator("form[action='/login'] input[name=_token]").inputValue();
const cookie = (await this.context.cookies(base.href)).find(item => item.name === "rullst_csrf");
if (!token || cookie?.value !== token) throw new Error("the form token does not match the rullst_csrf cookie");
if (!token || !cookie || cookie.value === token) throw new Error("the form token is not masked");
"#;

#[test]
fn a_browser_test_that_expects_the_cookie_in_the_form_is_reported() {
    // Line 4 throws unless they are equal; line 5 throws when they are, as
    // 13.0.0-alpha.6 expects; line 3 only finds the cookie by name.
    assert_eq!(js(BROWSER_SIGN_IN), [4]);
}

#[test]
fn equality_operators_report_outside_an_if_and_inequality_inside_one() {
    let source = r#"const csrf = (await context.cookies()).find(c => c.name === 'rullst_csrf');
const pageToken = await page.inputValue('input[name="_token"]');
check(pageToken === csrf.value, 'form token');
check(pageToken !== csrf.value, 'masked token');
} else if (pageToken != csrf.value) {
if (pageToken == csrf.value) {
const same = csrf?.value == pageToken ? 'same' : 'masked';
"#;
    // `csrf` holds the cookie because it was read from the cookie jar.
    assert_eq!(js(source), [3, 5, 7]);
}

#[test]
fn equality_assertions_are_reported_unless_negated() {
    let source = r#"import { expect } from '@playwright/test';
const cookies = await context.cookies();
const token = await page.locator('input[name=_token]').inputValue();
expect(token).toBe(cookies[0].value);
expect(token).not.toBe(cookies[0].value);
await expect(page.locator('input[name=_token]')).toHaveValue(cookie.value);
assert.equal(cookie.value, token);
assert.strictEqual(token, csrfCookie);
assert.notEqual(cookie.value, token);
expect(token).toHaveLength(86);
expect(response.status()).toBe(200);
"#;
    assert_eq!(js(source), [4, 6, 7, 8]);
}

#[test]
fn checks_that_a_page_contains_the_cookie_value_are_reported() {
    let source = r#"const cookie = jar.find(c => c.name === 'rullst_csrf');
const html = await response.text();
expect(html).toContain(cookie.value);
expect(html).not.toContain(cookie.value);
if (!html.includes(cookie.value)) throw new Error('no csrf token');
if (html.includes(cookie.value)) throw new Error('raw csrf token in the page');
assert(html.includes(cookie.value));
expect(setCookie).toContain('rullst_csrf=');
assert(response.headers['set-cookie'].includes(cookie.value));
"#;
    // A `Set-Cookie` header (line 9) is a cookie, not a page.
    assert_eq!(js(source), [3, 5, 7]);
}

#[test]
fn sending_the_cookie_value_is_not_a_comparison() {
    let source = r#"// CSRF: the middleware accepts the cookie's value as the token.
const csrf = document.cookie.split(';').map(v => v.trim()).find(v => v.startsWith('rullst_csrf='))?.slice(12) || '';
const r = await fetch('/api/x', { method: 'POST', headers: { 'x-csrf-token': csrf }, body: '{}' });
check((await evaluate(`fetch('/admin',{method:'POST',headers:{'x-csrf-token':${csrf}}}).then(r=>r.status)`)) === 405, 'read-only');
form.append('_token', cookie.value);
"#;
    assert_eq!(js(source), Vec::<usize>::new());
}

#[test]
fn comments_and_strings_never_compare() {
    let source = r#"/* The old check was
   cookie.value === token, which alpha.6 breaks. */
// expect(token).toBe(cookie.value);
const note = "cookie.value === token";
const page = `
  ${cookie.value === token}
`;
const after = cookie.value === token; // still code
"#;
    assert_eq!(js(source), [8]);
}

#[test]
fn python_smoke_tests_are_read_with_python_syntax() {
    let source = r#""""A smoke test: CSRF cookie and form token.

assert cookie == token in a docstring is text.
"""
def csrf(self):
    token = next((c.value for c in self.jar if c.name == "rullst_csrf"), "")
    return token

fields["_token"] = self.csrf()  # the raw cookie value is accepted
page_token = re.search(r'name="_token" value="([^"]+)"', body).group(1)
assert page_token == token
assert page_token != token
if page_token != token:
    raise AssertionError("token mismatch")
if page_token == token:
    raise AssertionError("raw token")
self.assertEqual(page_token, csrf_cookie)
self.assertNotEqual(page_token, csrf_cookie)
assert csrf_cookie in body
assert csrf_cookie not in body
if csrf_cookie not in body:
    raise AssertionError("missing")
for cookie in self.jar:
    pass
values = [cookie.value for cookie in jar if cookie.name == "rullst_csrf"]
assert "rullst_csrf=" in response.headers["set-cookie"]
# assert page_token == token
"#;
    // `token` was assigned from the cookie jar on line 6.
    assert_eq!(py(source), [11, 13, 17, 19, 21]);
}

#[test]
fn a_name_reassigned_from_the_page_no_longer_holds_the_cookie() {
    let source = "let token = cookie.value;\ntoken = await page.inputValue('input[name=_token]');\nassert.equal(token, pageToken);\n";
    assert_eq!(js(source), Vec::<usize>::new());
    let typed = "const value: string = cookies.find(c => c.name === 'rullst_csrf')!.value;\nexpect(pageToken).toBe(value);\n";
    assert_eq!(js(typed), [2]);
}

#[test]
fn scripts_below_the_project_are_found_outside_build_and_dependency_folders() {
    let project = Project::new();
    project
        .write("scripts/browser/browser.mjs", BROWSER_SIGN_IN)
        .write("e2e/tests/auth/login.spec.ts", BROWSER_SIGN_IN)
        .write(
            "scripts/smoke_test.py",
            "token = cookie.value  # csrf\nassert page_token == token\n",
        )
        .write("node_modules/lib/index.js", BROWSER_SIGN_IN)
        .write("target/debug/check.js", BROWSER_SIGN_IN)
        .write(
            ".venv/lib/check.py",
            "assert page_token == cookie  # csrf\n",
        )
        .write("static/app.min.js", BROWSER_SIGN_IN)
        .write("scripts/check.rb", BROWSER_SIGN_IN)
        .write(
            "scripts/session.js",
            "assert(sessionCookie === apiToken);\n",
        )
        .write("a/b/c/d/e/deep.js", BROWSER_SIGN_IN);
    assert_eq!(
        csrf_comparisons(project.root()),
        [
            ("e2e/tests/auth/login.spec.ts".to_string(), 4),
            ("scripts/browser/browser.mjs".to_string(), 4),
            ("scripts/smoke_test.py".to_string(), 2),
        ]
    );
    let found = project.findings("[package]\nname = \"app\"\n");
    assert!(
        found.contains(&(
            "scripts/browser/browser.mjs".to_string(),
            "V13-CSRF-TOKEN-COMPARED",
            4
        )),
        "{found:?}"
    );
}
