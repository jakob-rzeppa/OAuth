# auth-server: manual testing

Requests in flow order. Paste `<REQUEST_URI>`, `<CODE>` and `<ACCESS_TOKEN>` from the previous responses.

The `request_uri` expires after 3 minutes and the code after 5 minutes. Both can be used only once.

## Health

`GET http://localhost:8081/health`

## PAR

`POST http://localhost:8081/par`

Body (form, `application/x-www-form-urlencoded`):

```
client_id=75937f94-eaa9-485c-997e-f93f339f5f59&redirect_uri=http%3A%2F%2Fexample.com&response_type=code&scope=read%20write&state=manual-test-state&code_challenge=SUqfUdBKsRh7fZquwd-cCvI8iG9v-ascNovGCKRsJo4&code_challenge_method=S256
```

## Authorize page

`GET http://localhost:8081/authorize?client_id=75937f94-eaa9-485c-997e-f93f339f5f59&request_uri=<REQUEST_URI>`

## Authorize submit

`POST http://localhost:8081/authorize`

Body (form, `application/x-www-form-urlencoded`), approve:

```
client_id=75937f94-eaa9-485c-997e-f93f339f5f59&request_uri=<REQUEST_URI>&decision=true
```

Deny:

```
client_id=75937f94-eaa9-485c-997e-f93f339f5f59&request_uri=<REQUEST_URI>&decision=false
```

## Token

`POST http://localhost:8081/token`

Body (form, `application/x-www-form-urlencoded`):

```
grant_type=authorization_code&client_id=75937f94-eaa9-485c-997e-f93f339f5f59&code=<CODE>&code_verifier=manual-testing-code-verifier-0123456789abcdefghijklmnop
```

## Introspection

`POST http://localhost:8081/introspect`

Body (form, `application/x-www-form-urlencoded`):

```
token=<ACCESS_TOKEN>
```
