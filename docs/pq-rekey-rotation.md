# Design: fresh ML-KEM keys per PQ rekey

Status: proposal. Not implemented. Changes the wire format, so it needs the
maintainer's sign-off before code.

## The gap

`Session` keeps one ML-KEM decapsulation key (`kyber_longterm`) for the whole
session. It is created for the handshake and never replaced; the code comment
on the field says it is "stable for the session lifetime". Every PQ rekey
encapsulates to the same peer key (`kyber_rekey_initiate` uses
`peer_kyber_ek`) and the receiver decapsulates with that same key.

Consequence: an adversary who copies the session state once, at time t0,
holds both the ratchet state and the decapsulation key. Every later rekey
ciphertext they observe decapsulates to the same `k_new` the honest peer
derives. The rekey mixes nothing the adversary lacks.

What that breaks, precisely:

| Adversary | Heals after t0? | Why |
|---|---|---|
| Classical, passive | Yes | The X25519 ratchet step mixes a fresh DH output they cannot compute. |
| Quantum, passive, one state copy | **No** | They compute the DH outputs from recorded public values, and the PQ contribution is decapsulable with the stolen key. |

The second row is the property the README names "post-quantum healing".
The Tamarin lemma that would state it, `pcs_after_rekey` in
`model/ratchet.spthy`, is not proven today. Given this design it should not
be provable, so fix the protocol before spending proof effort.

## Proposed protocol: a KEM ratchet beside the DH ratchet

Each side owns a current decapsulation key and advertises a fresh
encapsulation key inside every rekey message. A decapsulation key is used for
exactly one inbound rekey and is then retired.

State per side: `dk_cur`/`ek_cur` (advertised to the peer, used for the next
inbound rekey), `dk_next` (generated for the next advertisement), and
`peer_ek` (the peer's latest advertised key).

Rekey from A to B:

1. A encapsulates to `peer_ek` (B's advertised key): `(ct, k_new)`.
2. A generates a fresh pair `(ek_A', dk_A')` and sends `(ct, ek_A')`.
3. A derives `root' = HKDF(root, k_new, H(ek_A' || peer_ek || ct))` so the
   new key and both encapsulation keys are bound into the transcript.
4. B decapsulates with the `dk` matching `peer_ek`, derives the same `root'`,
   stores `ek_A'` as its new `peer_ek`, and starts advertising a fresh key in
   its own next rekey.
5. A decapsulation key is zeroized once a rekey encrypted to its successor
   has been received, or when it falls out of the existing
   `RETAINED_REKEYS = 8` window. The retention window is what keeps
   out-of-order and replayed rekey events working.

After step 5 an adversary holding the t0 state can no longer decapsulate
rekeys addressed to later keys, so the PQ contribution is secret again.
That is the same shape as the DH ratchet, with a KEM in place of DH.

## Wire and size impact

- Rekey events gain one ML-KEM-1024 encapsulation key: +1568 bytes (the
  ciphertext is already 1568 bytes). Check the fixed 2048-byte frame budget:
  a rekey event no longer fits in one frame and needs fragmentation, or the
  key rides in a dedicated control frame.
- Rekey cadence is unchanged (`KYBER_REKEY_INTERVAL_MSGS = 50`,
  `KYBER_REKEY_INTERVAL_SECS = 7 days`), so the bandwidth cost is about
  1.5 KiB per 50 messages.
- Handshake is unchanged: the handshake key becomes the first `ek_cur`.
- Version the rekey frame, refuse downgrade to the old format, and
  regenerate the KAT vectors (`cargo xtask kat`).

## Work items

1. `null-crypto`: key rotation in `Session`, retired-key zeroization, the
   transcript-bound KDF, retained-event format with the advertised key.
2. `null-session`: fragment or carry the extra key; resend path
   (`rekey_events_since`) returns the new event shape.
3. Tests, written first:
   - state copy at t0, one rekey exchange in each direction, assert the copy
     cannot derive the new root even when handed the DH outputs (this is the
     quantum-passive adversary);
   - out-of-order, duplicated and dropped rekeys across a rotation boundary;
   - substituted advertised key is rejected (transcript binding).
4. `model/ratchet.spthy`: add a state-compromise rule plus a DH-reveal rule
   (the quantum abstraction), then prove `pcs_after_rekey`. Expect this to
   fail on the current protocol and pass on the proposed one.
5. Docs: README and `docs/security-posture.md` keep "PQ healing" labeled
   unproven until step 4 is green.

## Rejected alternatives

- **Rotate only on handshake.** Defeats the point of in-session rekeys.
- **Keep one key, shorten the interval.** Narrows the window but never closes
  it; a state copy still decapsulates everything afterwards.
- **Drop the PQ healing claim.** Honest and cheap, and the right fallback if
  the wire change is not worth it. The README must then say the PQ component
  protects confidentiality against harvest-now-decrypt-later, not healing.
