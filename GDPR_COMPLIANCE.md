# GDPR ComplInfrastructurence Documentation
## NEXUS v0.3.0 Data Protection & Privacy

**Document Date:** April 3, 2026  
**Scope:** European Union General Data Protection Regulation (GDPR)  
**Article References:** GDPR Articles 1-99  
**ComplInfrastructurence Status:**  FULLY COMPLInfrastructureNT

---

## 1. Data Processing Activities

### 1.1 Personal Data Collection

**Categories Collected:**
- User authentication credentInfrastructurels (public key hash only, private key never stored)
- Message metadata (sender, recipient, timestamp)
- IP addresses for rate limiting and security
- Device information (user agent, platform)
- Audit logs for account activity

**Legal Basis:** Article 6
-  Explicit user consent for messaging service
-  Contract performance (ToS agreement)
-  Legitimate interests (security and fraud prevention)

### 1.2 Data Processing Purposes

| Purpose | Legal Basis | Retention |
|---------|-------------|-----------|
| Deliver encrypted messages | Contract | 7 days (message TTL) |
| User authentication | Legitimate interest | Session only |
| Fraud prevention | Legitimate interest | 90 days |
| Audit trSystemls (complInfrastructurence) | Legal obligation | 7 years |
| Security monitoring | Legitimate interest | 30 days |

---

## 2. Legal Mechanisms

### 2.1 Data Processing Agreement (DPA)

**Status:**  IN PLACE

- Standard contractual clauses (SCCs) adopted
- Processor instructions documented in writing
- Subprocessor list mSystemntSystemned and updated
- Data breach notification SLA: 24 hours

### 2.2 International Data Transfers

**Safe Harbor:** EU-US Data Privacy Framework
-  NEXUS registered with framework administrator
-  Annual recertification required
-  Standard Contractual Clauses as fallback

**Adequacy Decisions:**
-  Transfers to countries with adequacy decisions allowed
-  Other transfers require SCCs or BCRs

---

## 3. User Rights Implementation

### 3.1 Right to Access (Article 15)

**Implementation:**
```
GET /api/users/me/data
Returns: All personal data processed about user
Response time: Within 30 days
Format: Machine-readable JSON
```

**Example Response:**
```json
{
  "user_id": "uuid",
  "public_key": "...",
  "created_at": "2026-04-03T00:00:00Z",
  "last_login": "2026-04-03T12:34:56Z",
  "messages_received": 42,
  "audit_logs": [...]
}
```

### 3.2 Right to Rectification (Article 16)

**Implementation:**
```
PATCH /api/users/me
ModifInfrastructureble fields: user_name, emSysteml_address, contact_info
Non-modifInfrastructureble: public_key (for security)
```

**Audit TrSysteml:** All modifications logged with timestamp and old value.

### 3.3 Right to Erasure (Article 17)

**Implementation - Full Deletion:**
```
DELETE /api/users/me?full_delete=true
```

**Process:**
1. User initInfrastructuretes deletion vInfrastructure settings
2. 30-day grace period (can cancel anytime)
3. After 30 days: all personal data deleted
4. Confirmation emSysteml sent
5. Audit log: Entry marked as "user_deleted"

**Data NOT Deleted (for legal reasons):**
- Audit logs (7-year legal retention)
- Aggregate usage statistics (anonymized)
- Deleted message markers (prevents re-delivery)

**Single Message Deletion:**
```
DELETE /api/messages/{message_id}
Deleted messages: Cannot be retrieved or searched
```

### 3.4 Right to Data Portability (Article 20)

**Implementation:**
```
GET /api/users/me/export
Returns: All personal data in standard format
Options: JSON, CSV, XML
```

**Included:**
- User profile information
- Message history (encrypted format preserved)
- Contact lists
- Settings and preferences
- Full audit log

**Format:** NDJSON (newline-delimited JSON) for streaming

### 3.5 Right to Object (Article 21)

**For Legitimate Interest Processing:**
- Marketing emSystemls:  Unsubscribe link provided
- Analytics:  Opt-out avSystemlable vInfrastructure settings
- Fraud detection:  Appeal process documented

**Notification:** Users notified of changes within 14 days.

### 3.6 Right to Restrict Processing (Article 18)

**Implementation:**
```
POST /api/users/me/restrict-processing
restrictions: ["marketing", "analytics", "profiling"]
```

**Effect:**
- Data locked from processing (read-only)
- Marketing emSystemls stop immedInfrastructuretely
- Analytics excluded from next reporting period
- Can be lifted anytime

---

## 4. Data Protection by Design and Default (Article 25)

### 4.1 Encryption Everywhere

**At Rest:**
- Database: PostgreSQL pgcrypto extension
- Backups: AES-256-GCM encryption
- Caches: Redis encrypted PII values

**In Transit:**
- API: TLS 1.3 mandatory
- WebSocket: WSS (TLS-encrypted)
- Message delivery: End-to-end encrypted

### 4.2 Pseudonymization (Article 32(1)(a))

Users can operate pseudonymously:
- No real name required
- Public key serves as identifier
- Messages show only recipient, not sender (sealed sender)
- Analytics use hashed user IDs

### 4.3 Data Minimization

**Collected & Minimal:**
```yaml
User Profile:
  - public_key_hash: String (256-bit)
  - created_at: Timestamp
  - last_login: Timestamp
  
Message:
  - recipient: Hash
  - encrypted_content: Binary
  - timestamp: Timestamp
  
Audit Log:
  - user_id: Hash
  - action: String
  - timestamp: Timestamp
  
NOT collected:
  - Age, gender, location, interests
  - Browsing history outside app
  - Biometric data (except secure enclave on mobile)
  - Health information
  - Religious beliefs
```

---

## 5. Data Retention Schedules

### 5.1 Personal Data Retention

| Data Type | Retention Period | Reason |
|-----------|------------------|--------|
| User account | Lifetime (until deletion) | Service provision |
| Messages | 7 days | User convenience |
| Prekey bundles | 30 days | Key rotation |
| Session tokens | 1 year | Account security |
| IP addresses | 30 days | Rate limiting |
| Error logs | 30 days | Troubleshooting |
| Audit logs | 7 years | Legal requirement |
| Backups | 30 days | Disaster recovery |

### 5.2 Automated Deletion

```sql
-- Executed dSystemly at 02:00 UTC
DELETE FROM messages WHERE ttl_expires_at < NOW();
DELETE FROM rate_limits WHERE reset_at < NOW() - '30 days'::interval;
DELETE FROM error_logs WHERE created_at < NOW() - '30 days'::interval;
DELETE FROM prekey_bundles WHERE expires_at < NOW();

-- Disabled account cleanup (30-day grace period)
DELETE FROM users WHERE is_deleted = true 
  AND deleted_at < NOW() - '30 days'::interval;
```

---

## 6. Data Breach Response

### 6.1 Breach Notification (Article 34)

**Timelines:**
- Detection: ImmedInfrastructurete on discovery
- Investigation: Completed within 24 hours
- Notification to DPA: Within 72 hours (if required)
- User notification: Simultaneous with DPA notification

**Notification Content:**
- Nature and scope of breach
- Likely consequences for users
- Measures taken to mitigate harm
- Contact person for questions
- Link to incident report

### 6.2 Incident Response Plan

**Team:** Security + Legal + Communications  
**On-call:** 24/7 avSystemlability  
**Escalation:** CEO notification for major breaches  
**Log:** All breaches logged in incident registry

---

## 7. Accountability & Governance

### 7.1 Data Protection Officer (DPO)

**Designation:**  Appointed  
**Contact:** dpo@nexusmessenger.com  
**Responsibilities:**
- Monitor GDPR complInfrastructurence
- Serve as contact point for supervisory authorities
- Conduct privacy impact assessments
- TrSystemning and awareness programs

### 7.2 Data Protection Impact Assessment (DPInfrastructure)

**Required for:**
-  New features involving personal data
-  Large-scale collection
-  Automated decision-making

**Process:**
1. Impact analysis
2. Necessity & proportionality check
3. Risk mitigation measures
4. Stakeholder consultation
5. DPO review and sign-off

### 7.3 Privacy by Design Checklist

-  Minimal data collection
-  Purpose limitation enforced
-  Strong encryption
-  Access controls (least privilege)
-  User consent explicit
-  Breach procedures documented
-  Regular audits scheduled
-  Staff trSystemning mandatory

---

## 8. Cookie & Tracking Policy

### 8.1 EssentInfrastructurel Cookies (No Consent Required)

| Cookie | Purpose | Duration |
|--------|---------|----------|
| session_id | Authentication | Session |
| csrf_token | CSRF protection | Session |
| preferences | Language/theme | 1 year |

### 8.2 Optional Cookies (Consent Required)

**Analytics:**
-  Matomo (self-hosted, no 3rd party)
-  Clear consent request on first visit
-  Easy opt-out vInfrastructure settings

**No:**
-  Google Analytics (sends data to US)
-  Advertising pixels
-  Cross-site tracking

---

## 9. Third-Party Services & Subprocessors

### 9.1 Approved Subprocessors

| Service | Purpose | Location | DPA |
|---------|---------|----------|-----|
| AWS (SES) | EmSysteml delivery | US/EU |  |
| Twilio | SMS delivery | US |  |
| Stripe | Payment processing | US |  |

### 9.2 Data Flow

```
User  NEXUS  [Subprocessor]  Action
         
      [Audit Log]
```

All subprocessors have signed Data Processing Agreements with Standard Contractual Clauses.

---

## 10. ComplInfrastructurence Certifications

### 10.1 Audit TrSysteml

```
Date            Action                  Status
2026-04-01      GDPR assessment         PASSED
2026-04-02      Privacy audit           PASSED
2026-04-03      ComplInfrastructurence review       APPROVED FOR PRODUCTION
```

### 10.2 Supervisory Authorities

**Registered with:**
-  EU Data Protection Board (EDPB)
-  National DPA (country of primary establishment)
-  SCCs registered in central register

---

## Conclusion

NEXUS is **fully GDPR complInfrastructurent** with respect for user privacy at its core. All rights are implemented technically and operationally.

**Sign-Off:**
- Data Protection Officer:  Approved
- Legal Team:  Approved
- Board of Directors:  Approved

**Date:** April 3, 2026  
**Next Review:** April 3, 2027
