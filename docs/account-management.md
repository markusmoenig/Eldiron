# Online account management

Persistent hosts support authenticated password changes and **Log out everywhere**. Accountless demos stay accountless. Single player and Creator Play keep their embedded sessions and do not open an account service.

In the canonical graphical Player, log in, then choose **Account** in the connection screen. While playing, **F10** opens the same account settings. Authored UI can use `game.account`, `game.account_back`, `game.change_password` and `game.logout_all`; existing `game.logout` still signs out the current connection. Back returns to the game when a character is active. Opening settings cancels held movement through the existing screen-navigation path; it does not pause the authoritative world.

The password form uses masked current/new/confirmation fields. New passwords must contain 12–256 UTF-8 bytes and match the confirmation. The fields and pending clipboard ownership are cleared on submission, navigation away and disconnect. These values belong to runtime UI state; they never enter project exports, characters or public replication. Account controls require the persistent server's advertised capability and an authenticated session. Existing servers without that capability remain playable and hide the new controls.

Authored inputs use `online.current_password`, `online.new_password` and `online.confirm_password`, with `secret = true`. The built-in form installs those bindings only in the running Player copy.

## Password changes and revocation

`ChangePassword` supplies only the current/new secrets; it cannot choose an account ID. The serving socket's authenticated identity selects the account. The shared four-job Argon2id worker limit and existing per-IP authentication budget apply. Current-password verification and new-password hashing run outside the simulation owner. A failed current-password check leaves the account and its session authorized; no hash or revision is changed.

Successful password changes commit the replacement hash and incremented account security revision before reporting success. The host then revokes all resume tokens and disconnects all account sockets, including the requester. Log in again with the new password. If the success reply is lost, try a new password login; credentials are not rolled back to match a lost reply.

**Log out everywhere** commits a security-revision increment, revokes all account tokens and disconnects its sockets. It requires an authenticated session but does not change the password. The current one-live-socket-per-account policy remains; the command also invalidates disconnected resume credentials and pending older login verifications.

Hash workers capture the account's security revision. The world owner checks it again before accepting a verified login or password update. Thus an authentication job begun before revocation cannot restore access afterward. Resume tokens also carry the revision and retain their existing rotation and one-hour lifetime. All authorization state remains per-world; another game/world's account service cannot revoke this world's users.

Account IDs and character ownership do not change. Disconnecting stops control while retaining the saved body and inventory under the existing policy. Account management does not add combat-logout/despawn behavior. Security metadata commits preserve creation receipts and action journals without consuming the bounded character-creation receipt ledger. Storage ambiguity stops the owner and requires reloading the committed head; no success or new in-memory directory is published before commit.

Native and browser connection workers drain already-verified ordered replies before reporting socket closure, within a ten-second drain limit. This allows the final account acknowledgment to reach the Player even when the authority closes immediately afterward. Backpressure and required-content ordering remain bounded; a prolonged stall still reports an explicit connection failure.

## Private operator recovery

For a forgotten password, use the offline `eldiron-world reset-password` command. There is no public reset endpoint or email-based recovery service. The operator verifies the user's ownership through their own trusted support process, stops the world owner, and selects the matching private project/save. Then pipe one new password line from a password manager or other private input source:

```sh
PASSWORD_SOURCE | cargo run -p eldiron-server --bin eldiron-world -- \
  reset-password --source private/world.save --project private/game.eldiron \
  --username alice --password-stdin --output private/recovered.save
```

`PASSWORD_SOURCE` stands for the operator's password-producing command. Do not replace it with a literal password in shell history. The tool refuses interactive echoed stdin, password arguments and an existing output path. It accepts exactly one UTF-8 line, strips only its final LF/CRLF, and does not print the password or hash.

Recovery writes a fresh private snapshot with the same world/account/character/item identities, creation receipts and action streams, a new password hash and a higher security revision. It never edits the source save. Deploy the recovered snapshot as the stopped world's new save slot using the existing restore procedure, then start the matching server. Resetting a copy does not revoke a separate old process that remains running; restart/switch the intended owner. Restart already discards all in-memory resume tokens.

This tool requires a dedicated save, an existing canonical username and a matching project fingerprint. It uses the existing fresh-path atomic snapshot writer and owner-only file permissions. Keep projects, saves, backups and account hashes outside public web roots. The tool prints only the ordinary sanitized snapshot summary.

## Compatibility and hosting

WSS, subprotocol `eldiron.v4`, replication DTOs and save containers are unchanged. Account messages are additive and used only after the persistent server advertises `account_management` in Welcome; absent capabilities default to false. Existing single-file and streamed public client releases retain their canonical gameplay identities. Private `accounts.v1` records add a default-zero `security_revision`, so previous saved account directories still load.

A future login provider can attach credentials to the same account UUID. It must honor the account security revision when publishing authentication results; ownership remains independent of the provider. A managed control plane can wrap the private recovery operation under its own operator authorization, but no public administrator service, global identity namespace, email verification or external provider is introduced here.
