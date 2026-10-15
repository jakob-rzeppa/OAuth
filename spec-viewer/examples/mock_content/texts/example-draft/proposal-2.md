# Example Draft

## Overview

Device pairing lets a user sign in on a device with limited input, like a
TV or a printer, by confirming a short code on a second device they
already trust.

## Flow

1. The device requests a pairing code from the authorization server.
2. The device shows the code and a short URL to the user.
3. The user opens the URL on the second device, signs in and enters the
   code.
4. The authorization server shows which device is asking for access, so
   the user can deny a request they did not start.
5. The device polls the token endpoint until the user has confirmed.

## Pairing codes

A pairing code MUST be at least 8 characters long and MUST expire after
15 minutes. Codes SHOULD avoid characters that are easy to confuse, like
`0` and `O`.

## Polling

The device MUST wait at least 5 seconds between two polls. When the
server answers with `slow_down`, the device MUST add 5 seconds to the
interval.

## Security considerations

An attacker can show a victim their own pairing code and wait for the
victim to confirm it. The confirmation page MUST name the device and
the requested access clearly, and SHOULD warn when the code was not
entered within a few minutes of being shown.
