# Login (Planned)

Optional sign-in, from [SPEC](../specs/SPEC.md) section 8. Lands with M6. It unlocks age-gated, members-only and
private videos, Premium formats, and the library (Watch Later, Liked, library playlists, subscriptions). Public
videos keep the anonymous path even when signed in: login adds risk to the account and no help there.

## TV device-code sign-in

`sign-in = tv` turns it on.

1. The TV client id and secret are scraped from the `/tv` page's base script (the first `clientId` followed by a
   short string), kept on disk, re-scraped on a 401.
2. `POST https://www.youtube.com/o/oauth2/device/code` gives a `user_code`, a `device_code`, an expiry and a polling
   interval.
3. The user enters the code at youtube.com/activate. Halcyon polls `POST https://www.youtube.com/o/oauth2/token`
   at the given interval until it gets an access and a refresh token.
4. The access token (about an hour) is refreshed on demand, at most once per call. A revoked refresh token is
   deleted and the call answers `auth-failed`; a signed-in source never falls back to anonymous silently.
5. `Authorization: Bearer <token>` goes on Innertube calls with a TV client only.

## Showing the code

Through the `action-required(prompt)` error of Nightglass contract 0.4.0 (PR #46): `load-catalog` answers it with
the message, the `verification_url` and the `user_code` of the device-code answer, and `retry-in` set to the polling
interval. The Sources screen shows it under the source and asks again until the plugin answers something else. The spec's
workaround (the code in `catalog.account.status` and in a placeholder series) is not the plan. See
[decisions.md](../decisions.md).

## Brand accounts

`account/accounts_list` lists the owner and brand accounts; `X-Goog-Pageid: <pageId>` on every request acts as one.
The `account` field picks it by name or handle.

## Cookies, the alternative

A browser `Cookie` header in the `cookies` secret field: `SAPISIDHASH` authorization headers computed from it, web
and TV clients only, rotated cookies written back to storage (never to the field). Accounts that send automated
traffic can be banned; the field's placeholder says so.

## Storage

Client id and secret, a per-install device id and the refresh token on disk; the access token and a pending code in
memory, each for its own lifetime; the account list for a day. Keys carry a hash of the source's `sign-in` and
`account` so several sources can sign in to different accounts.

## Sources

None yet; lands with M6.
