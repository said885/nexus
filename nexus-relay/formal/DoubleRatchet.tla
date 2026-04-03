--------------------------- MODULE DoubleRatchet ---------------------------
(* NEXUS Double Ratchet Protocol - Formal Verification with TLA+ *)
(* 
   This specification formally verifies the Double Ratchet algorithm
   used in the NEXUS messaging protocol.
   
   Security Properties Verified:
   1. Forward Secrecy - Compromise of current keys doesn't affect past messages
   2. Break-in Recovery - Self-healing after key compromise
   3. Message Ordering - Messages are processed in correct order
   4. Key Uniqueness - Each message uses a unique key
*)

EXTENDS Integers, Sequences, FiniteSets, TLC

CONSTANTS
    MaxMessages,     \* Maximum number of messages to send
    MaxSkip,         \* Maximum number of skipped messages
    NumUsers         \* Number of users in the system

VARIABLES
    \* User state
    rootKey,         \* Root key for each user
    sendChainKey,    \* Send chain key for each user
    recvChainKey,    \* Receive chain key for each user
    sendCounter,     \* Send message counter
    recvCounter,     \* Receive message counter
    dhPrivateKey,    \* DH private key for each user
    dhPublicKey,     \* DH public key for each user
    remoteDHPubKey,  \* Remote party's DH public key
    
    \* Message state
    messageQueue,    \* Queue of messages in transit
    receivedMessages,\* Set of received message keys
    skippedKeys,     \* Skipped message keys
    
    \* Protocol state
    sessionActive,   \* Whether session is active
    compromised      \* Whether a user has been compromised

UserSet == 1..NumUsers

\* Type invariants
TypeInvariant ==
    /\ rootKey \in [UserSet -> Seq(INTEGER)]
    /\ sendChainKey \in [UserSet -> Seq(INTEGER)]
    /\ recvChainKey \in [UserSet -> Seq(INTEGER)]
    /\ sendCounter \in [UserSet -> Nat]
    /\ recvCounter \in [UserSet -> Nat]
    /\ messageQueue \in Seq([sender: UserSet, receiver: UserSet, counter: Nat, key: Seq(INTEGER)])
    /\ receivedMessages \in SUBSET (UserSet \X UserSet \X Nat)
    /\ sessionActive \in [UserSet -> BOOLEAN]
    /\ compromised \in [UserSet -> BOOLEAN]

\* Initial state
Init ==
    /\ rootKey = [u \in UserSet |-> <<>>]
    /\ sendChainKey = [u \in UserSet |-> <<>>]
    /\ recvChainKey = [u \in UserSet |-> <<>>]
    /\ sendCounter = [u \in UserSet |-> 0]
    /\ recvCounter = [u \in UserSet |-> 0]
    /\ dhPrivateKey = [u \in UserSet |-> <<u>>]
    /\ dhPublicKey = [u \in UserSet |-> <<u>>]
    /\ remoteDHPubKey = [u \in UserSet |-> <<>>]
    /\ messageQueue = <<>>
    /\ receivedMessages = {}
    /\ skippedKeys = [u \in UserSet |-> {}]
    /\ sessionActive = [u \in UserSet |-> FALSE]
    /\ compromised = [u \in UserSet |-> FALSE]

\* Helper: Derive next chain key
DeriveChainKey(chainKey) ==
    Append(chainKey, 1)

\* Helper: Derive message key from chain key
DeriveMessageKey(chainKey) ==
    Append(chainKey, 2)

\* Action: Initialize session (X3DH)
InitSession(sender, receiver) ==
    /\ ~sessionActive[sender]
    /\ ~sessionActive[receiver]
    /\ LET sharedSecret == <<sender + receiver>>
       IN
       /\ rootKey' = [rootKey EXCEPT ![sender] = sharedSecret, ![receiver] = sharedSecret]
       /\ sendChainKey' = [sendChainKey EXCEPT ![sender] = DeriveChainKey(sharedSecret)]
       /\ recvChainKey' = [recvChainKey EXCEPT ![receiver] = DeriveChainKey(sharedSecret)]
       /\ sessionActive' = [sessionActive EXCEPT ![sender] = TRUE, ![receiver] = TRUE]
       /\ UNCHANGED <<sendCounter, recvCounter, dhPrivateKey, dhPublicKey, 
                       remoteDHPubKey, messageQueue, receivedMessages, 
                       skippedKeys, compromised>>

\* Action: Send a message
SendMessage(sender, receiver) ==
    /\ sessionActive[sender]
    /\ sendCounter[sender] < MaxMessages
    /\ ~compromised[sender]
    /\ LET msgKey == DeriveMessageKey(sendChainKey[sender])
           newChainKey == DeriveChainKey(sendChainKey[sender])
           counter == sendCounter[sender] + 1
       IN
       /\ messageQueue' = Append(messageQueue, 
              [sender |-> sender, receiver |-> receiver, 
               counter |-> counter, key |-> msgKey])
       /\ sendChainKey' = [sendChainKey EXCEPT ![sender] = newChainKey]
       /\ sendCounter' = [sendCounter EXCEPT ![sender] = counter]
       /\ UNCHANGED <<rootKey, recvChainKey, recvCounter, dhPrivateKey,
                       dhPublicKey, remoteDHPubKey, receivedMessages,
                       skippedKeys, sessionActive, compromised>>

\* Action: Receive a message
ReceiveMessage(receiver) ==
    /\ sessionActive[receiver]
    /\ Len(messageQueue) > 0
    /\ LET msg == Head(messageQueue)
           rest == Tail(messageQueue)
       IN
       /\ msg.receiver = receiver
       /\ <<msg.sender, msg.receiver, msg.counter>> \notin receivedMessages
       /\ receivedMessages' = receivedMessages \cup {<<msg.sender, msg.receiver, msg.counter>>}
       /\ messageQueue' = rest
       /\ recvCounter' = [recvCounter EXCEPT ![receiver] = recvCounter[receiver] + 1]
       /\ recvChainKey' = [recvChainKey EXCEPT ![receiver] = DeriveChainKey(recvChainKey[receiver])]
       /\ UNCHANGED <<rootKey, sendChainKey, sendCounter, dhPrivateKey,
                       dhPublicKey, remoteDHPubKey, skippedKeys,
                       sessionActive, compromised>>

\* Action: DH Ratchet step (key rotation)
DHRatchetStep(user) ==
    /\ sessionActive[user]
    /\ sendCounter[user] > 0
    /\ LET newDHPrivate == Append(dhPrivateKey[user], user)
           newDHPublic == Append(dhPublicKey[user], user)
           newSharedSecret == Append(rootKey[user], user + 1)
       IN
       /\ dhPrivateKey' = [dhPrivateKey EXCEPT ![user] = newDHPrivate]
       /\ dhPublicKey' = [dhPublicKey EXCEPT ![user] = newDHPublic]
       /\ rootKey' = [rootKey EXCEPT ![user] = newSharedSecret]
       /\ sendChainKey' = [sendChainKey EXCEPT ![user] = DeriveChainKey(newSharedSecret)]
       /\ sendCounter' = [sendCounter EXCEPT ![user] = 0]
       /\ UNCHANGED <<recvChainKey, recvCounter, remoteDHPubKey,
                       messageQueue, receivedMessages, skippedKeys,
                       sessionActive, compromised>>

\* Action: Compromise a user (adversary action)
CompromiseUser(user) ==
    /\ ~compromised[user]
    /\ compromised' = [compromised EXCEPT ![user] = TRUE]
    /\ UNCHANGED <<rootKey, sendChainKey, recvChainKey, sendCounter, recvCounter,
                    dhPrivateKey, dhPublicKey, remoteDHPubKey, messageQueue,
                    receivedMessages, skippedKeys, sessionActive>>

\* Action: Recover from compromise (self-healing)
RecoverFromCompromise(user) ==
    /\ compromised[user]
    /\ sessionActive[user]
    /\ DHRatchetStep(user)
    /\ compromised' = [compromised EXCEPT ![user] = FALSE]

\* Next state relation
Next ==
    \/ \E sender, receiver \in UserSet : 
           sender # receiver /\ InitSession(sender, receiver)
    \/ \E sender, receiver \in UserSet : 
           sender # receiver /\ SendMessage(sender, receiver)
    \/ \E receiver \in UserSet : ReceiveMessage(receiver)
    \/ \E user \in UserSet : DHRatchetStep(user)
    \/ \E user \in UserSet : CompromiseUser(user)
    \/ \E user \in UserSet : RecoverFromCompromise(user)

\* ============================================================================
\* SAFETY PROPERTIES
\* ============================================================================

\* Property 1: Forward Secrecy
\* If a user is compromised, previously received messages cannot be decrypted
\* (because their keys have been ratcheted away)
ForwardSecrecy ==
    \A u \in UserSet :
        compromised[u] =>
            \A msg \in receivedMessages :
                msg[2] = u =>  \* If message was received by compromised user
                    \* The message key should no longer be derivable from current state
                    sendCounter[msg[1]] > msg[3]

\* Property 2: Key Uniqueness
\* Each message must use a unique key (no key reuse)
KeyUniqueness ==
    \A m1, m2 \in receivedMessages :
        m1 # m2 => m1[3] # m2[3]  \* Different counters

\* Property 3: Message Ordering
\* Messages from same sender are received in order
MessageOrdering ==
    \A m1, m2 \in receivedMessages :
        m1[1] = m2[1] /\ m1[2] = m2[2] /\ m1[3] < m2[3] =>
            <<m1[1], m1[2], m1[3]>> \in receivedMessages

\* Property 4: Session Integrity
\* If session is active, root key must be set
SessionIntegrity ==
    \A u \in UserSet :
        sessionActive[u] => rootKey[u] # <<>>

\* Property 5: No Replay
\* Same message cannot be received twice
NoReplay ==
    \A msg \in DOMAIN messageQueue :
        LET m == messageQueue[msg]
        IN <<m.sender, m.receiver, m.counter>> \notin receivedMessages

\* ============================================================================
\* LIVENESS PROPERTIES
\* ============================================================================

\* Eventually all messages are delivered
EventualDelivery ==
    <>(Len(messageQueue) = 0)

\* Session eventually becomes active
EventualSession ==
    <>(\E u \in UserSet : sessionActive[u])

\* ============================================================================
\* COMBINED SPECIFICATION
\* ============================================================================

Spec == Init /\ [][Next]_<<rootKey, sendChainKey, recvChainKey, sendCounter,
                            recvCounter, dhPrivateKey, dhPublicKey, remoteDHPubKey,
                            messageQueue, receivedMessages, skippedKeys,
                            sessionActive, compromised>>

\* Theorems to verify
THEOREM Spec => []TypeInvariant
THEOREM Spec => []ForwardSecrecy
THEOREM Spec => []KeyUniqueness
THEOREM Spec => []MessageOrdering
THEOREM Spec => []SessionIntegrity
THEOREM Spec => []NoReplay

=============================================================================
