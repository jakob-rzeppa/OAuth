# CSRF protection in the auth-server

The browser-facing form endpoints in the projects are protected against cross-site request forgery (CSRF) attacks using the following techniques:

1. `Sec-Fetch-Site` check must be `same-origin` or `none`.
2. If `Sec-Fetch-Site` is missing, the `Origin` header must match the auth-server's origin.
3. If neither header is present, the request is rejected with a 403 error page that asks the user to use an up-to-date browser.

This is secure because the browser sets `Sec-Fetch-Site` and `Origin` itself, and page scripts cannot override them. A cross-site attacker cannot make the victim's browser forge a same-origin value. For that reason, no CSRF tokens are used. The app is meant to be used only by modern browsers.
