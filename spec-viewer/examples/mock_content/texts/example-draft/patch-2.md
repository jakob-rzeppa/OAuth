# Example Draft

## Overview

Device pairing lets a user sign in on a device without a keyboard, like a
TV, by confirming a short code on their phone.

## Flow

1. The device requests a pairing code from the authorization server.
2. The device shows the code and a short URL to the user.
3. The user opens the URL on their phone, signs in and enters the code.
4. The device polls the token endpoint until the user has confirmed.

## Pairing codes

A pairing code MUST be at least 8 characters long and MUST expire after
15 minutes. Codes SHOULD avoid characters that are easy to confuse, like
`0` and `O`.

## Polling

The device MUST wait at least 5 seconds between two polls. When the
server answers with `slow_down`, the device MUST add 5 seconds to the
interval.
