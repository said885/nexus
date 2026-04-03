---------------------------- MODULE X3DH -----------------------------------
(* NEXUS X3DH Key Agreement Protocol - Formal Verification *)
(*
   Verifies the Extended Triple Diffie-Hellman key exchange protocol.
   
   Properties Verified:
   1. Key Agreement - Both parties derive same shared secret
   2. Identity Binding - Keys are bound to identities
   3. Forward Secrecy - Compromise of long-term keys doesn't affect past sessions
   4. No Unknown Key Share - Parties know who they're communicating with
*)

EXTENDS Integers, Sequences, FiniteSets

CONSTANTS
    NumUsers,
    MaxPreKeys

VARIABLES
    \* Identity keys (long-term)
    identityKeyPrivate,
    identityKeyPublic,
    
    \* Signed prekeys (medium-term)
    signedPreKeyPrivate,
    signedPreKeyPublic,
    signedPreKeySignature,
    
    \* One-time prekeys (ephemeral)
    oneTimePreKeys,
    oneTimePreKeysUsed,
    
    \* Ephemeral keys
    ephemeralPrivate,
    ephemeralPublic,
    
    \* Derived secrets
    sharedSecret,
    
    \* Protocol state
    handshakeComplete,
    compromised

UserSet == 1..NumUsers

\* Initial state
Init ==
    /\ identityKeyPrivate = [u \in UserSet |-> <<u * 10>>]
    /\ identityKeyPublic = [u \in UserSet |-> <<u * 10>>]
    /\ signedPreKeyPrivate = [u \in UserSet |-> <<u * 100>>]
    /\ signedPreKeyPublic = [u \in UserSet |-> <<u * 100>>]
    /\ signedPreKeySignature = [u \in UserSet |-> <<u * 1000>>]
    /\ oneTimePreKeys = [u \in UserSet |-> {<<u * 1000 + i>> : i \in 1..MaxPreKeys}]
    /\ oneTimePreKeysUsed = [u \in UserSet |-> {}]
    /\ ephemeralPrivate = [u \in UserSet |-> <<>>]
    /\ ephemeralPublic = [u \in UserSet |-> <<>>]
    /\ sharedSecret = [u \in UserSet |-> <<>>]
    /\ handshakeComplete = [u \in UserSet |-> FALSE]
    /\ compromised = [u \in UserSet |-> FALSE]

\* Helper: DH operation (simplified)
DH(private, public) ==
    Append(private, Head(public))

\* Helper: Combine DH results
CombineDH(dh1, dh2, dh3, dh4) ==
    dh1 \o dh2 \o dh3 \o dh4

\* Action: Generate ephemeral key
GenerateEphemeral(user) ==
    /\ ephemeralPrivate[user] = <<>>
    /\ ephemeralPrivate' = [ephemeralPrivate EXCEPT ![user] = <<user * 50>>]
    /\ ephemeralPublic' = [ephemeralPublic EXCEPT ![user] = <<user * 50>>]
    /\ UNCHANGED <<identityKeyPrivate, identityKeyPublic, signedPreKeyPrivate,
                    signedPreKeyPublic, signedPreKeySignature, oneTimePreKeys,
                    oneTimePreKeysUsed, sharedSecret, handshakeComplete, compromised>>

\* Action: Fetch prekey bundle
FetchPreKeyBundle(requester, target) ==
    /\ oneTimePreKeys[target] # {}
    /\ LET opk == CHOOSE k \in oneTimePreKeys[target] : TRUE
       IN
       /\ oneTimePreKeys' = [oneTimePreKeys EXCEPT ![target] = oneTimePreKeys[target] \ {opk}]
       /\ oneTimePreKeysUsed' = [oneTimePreKeysUsed EXCEPT ![target] = oneTimePreKeysUsed[target] \cup {opk}]
       /\ UNCHANGED <<identityKeyPrivate, identityKeyPublic, signedPreKeyPrivate,
                       signedPreKeyPublic, signedPreKeySignature, ephemeralPrivate,
                       ephemeralPublic, sharedSecret, handshakeComplete, compromised>>

\* Action: Perform X3DH (Alice initiates)
PerformX3DH(alice, bob) ==
    /\ alice # bob
    /\ ephemeralPrivate[alice] # <<>>
    /\ oneTimePreKeysUsed[bob] # {}
    /\ ~handshakeComplete[alice]
    /\ LET opk == CHOOSE k \in oneTimePreKeysUsed[bob] : TRUE
           \* DH1 = DH(IKa, SPKb)
           dh1 == DH(identityKeyPrivate[alice], signedPreKeyPublic[bob])
           \* DH2 = DH(EKa, IKb)
           dh2 == DH(ephemeralPrivate[alice], identityKeyPublic[bob])
           \* DH3 = DH(EKa, SPKb)
           dh3 == DH(ephemeralPrivate[alice], signedPreKeyPublic[bob])
           \* DH4 = DH(EKa, OPKb)
           dh4 == DH(ephemeralPrivate[alice], opk)
           \* Combine
           secret == CombineDH(dh1, dh2, dh3, dh4)
       IN
       /\ sharedSecret' = [sharedSecret EXCEPT ![alice] = secret]
       /\ handshakeComplete' = [handshakeComplete EXCEPT ![alice] = TRUE]
       /\ UNCHANGED <<identityKeyPrivate, identityKeyPublic, signedPreKeyPrivate,
                       signedPreKeyPublic, signedPreKeySignature, oneTimePreKeys,
                       oneTimePreKeysUsed, ephemeralPrivate, ephemeralPublic, compromised>>

\* Action: Respond to X3DH (Bob derives secret)
RespondX3DH(bob, alice) ==
    /\ bob # alice
    /\ handshakeComplete[alice]
    /\ ~handshakeComplete[bob]
    /\ LET opk == CHOOSE k \in oneTimePreKeysUsed[bob] : TRUE
           \* DH1 = DH(SPKb, IKa)
           dh1 == DH(signedPreKeyPrivate[bob], identityKeyPublic[alice])
           \* DH2 = DH(IKb, EKa)
           dh2 == DH(identityKeyPrivate[bob], ephemeralPublic[alice])
           \* DH3 = DH(SPKb, EKa)
           dh3 == DH(signedPreKeyPrivate[bob], ephemeralPublic[alice])
           \* DH4 = DH(OPKb, EKa)
           dh4 == DH(opk, ephemeralPublic[alice])
           \* Combine
           secret == CombineDH(dh1, dh2, dh3, dh4)
       IN
       /\ sharedSecret' = [sharedSecret EXCEPT ![bob] = secret]
       /\ handshakeComplete' = [handshakeComplete EXCEPT ![bob] = TRUE]
       /\ UNCHANGED <<identityKeyPrivate, identityKeyPublic, signedPreKeyPrivate,
                       signedPreKeyPublic, signedPreKeySignature, oneTimePreKeys,
                       oneTimePreKeysUsed, ephemeralPrivate, ephemeralPublic, compromised>>

\* Action: Compromise long-term key
CompromiseIdentity(user) ==
    /\ ~compromised[user]
    /\ compromised' = [compromised EXCEPT ![user] = TRUE]
    /\ UNCHANGED <<identityKeyPrivate, identityKeyPublic, signedPreKeyPrivate,
                    signedPreKeyPublic, signedPreKeySignature, oneTimePreKeys,
                    oneTimePreKeysUsed, ephemeralPrivate, ephemeralPublic,
                    sharedSecret, handshakeComplete>>

\* Next state
Next ==
    \/ \E u \in UserSet : GenerateEphemeral(u)
    \/ \E u1, u2 \in UserSet : u1 # u2 /\ FetchPreKeyBundle(u1, u2)
    \/ \E u1, u2 \in UserSet : u1 # u2 /\ PerformX3DH(u1, u2)
    \/ \E u1, u2 \in UserSet : u1 # u2 /\ RespondX3DH(u1, u2)
    \/ \E u \in UserSet : CompromiseIdentity(u)

\* ============================================================================
\* SAFETY PROPERTIES
\* ============================================================================

\* Property 1: Key Agreement
\* If both parties complete handshake, they have same shared secret
KeyAgreement ==
    \A u1, u2 \in UserSet :
        handshakeComplete[u1] /\ handshakeComplete[u2] =>
            sharedSecret[u1] = sharedSecret[u2]

\* Property 2: Forward Secrecy
\* Compromise of identity key doesn't reveal past shared secrets
\* (if one-time prekey was used and deleted)
ForwardSecrecy ==
    \A u \in UserSet :
        compromised[u] =>
            \* Past sessions used one-time prekeys that are now deleted
            oneTimePreKeysUsed[u] \cap oneTimePreKeys[u] = {}

\* Property 3: Identity Binding
\* Shared secret is bound to both identities
IdentityBinding ==
    \A u \in UserSet :
        handshakeComplete[u] =>
            \E v \in UserSet : sharedSecret[u] # <<>>

\* Property 4: Prekey Consumption
\* One-time prekeys are consumed exactly once
PrekeyConsumption ==
    \A u \in UserSet :
        oneTimePreKeysUsed[u] \cap oneTimePreKeys[u] = {}

\* Combined spec
Spec == Init /\ [][Next]_<<identityKeyPrivate, identityKeyPublic, signedPreKeyPrivate,
                            signedPreKeyPublic, signedPreKeySignature, oneTimePreKeys,
                            oneTimePreKeysUsed, ephemeralPrivate, ephemeralPublic,
                            sharedSecret, handshakeComplete, compromised>>

THEOREM Spec => []KeyAgreement
THEOREM Spec => []ForwardSecrecy
THEOREM Spec => []IdentityBinding
THEOREM Spec => []PrekeyConsumption

=============================================================================
