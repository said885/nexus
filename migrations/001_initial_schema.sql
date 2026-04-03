-- NEXUS Database Schema
-- Post-quantum secure messaging platform

-- Enable extensions
CREATE EXTENSION IF NOT EXISTS pgcrypto;
CREATE EXTENSION IF NOT EXISTS uuid-ossp;

-- Users table
CREATE TABLE users (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  public_key_hash VARCHAR(128) UNIQUE NOT NULL,
  public_key TEXT NOT NULL,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
  last_seen TIMESTAMP WITH TIME ZONE,
  is_verified BOOLEAN DEFAULT FALSE,
  verification_token VARCHAR(255),
  verification_expires_at TIMESTAMP WITH TIME ZONE
);

-- Prekey bundles for X3DH
CREATE TABLE prekey_bundles (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  identity_key VARCHAR(255) NOT NULL,
  signed_prekey VARCHAR(255) NOT NULL,
  signed_prekey_signature VARCHAR(255) NOT NULL,
  one_time_prekeys TEXT NOT NULL, -- JSON array
  created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
  expires_at TIMESTAMP WITH TIME ZONE
);

-- Messages table (encrypted)
CREATE TABLE messages (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  sender_id UUID NOT NULL REFERENCES users(id),
  recipient_id UUID NOT NULL REFERENCES users(id),
  sealed_content BYTEA NOT NULL,
  nonce VARCHAR(255) NOT NULL,
  auth_tag VARCHAR(255) NOT NULL,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
  delivered_at TIMESTAMP WITH TIME ZONE,
  read_at TIMESTAMP WITH TIME ZONE,
  ttl_expires_at TIMESTAMP WITH TIME ZONE,
  is_deleted BOOLEAN DEFAULT FALSE
);

-- Group messages
CREATE TABLE group_messages (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  group_id UUID NOT NULL,
  sender_id UUID NOT NULL REFERENCES users(id),
  sealed_content BYTEA NOT NULL,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
  delivered_count INT DEFAULT 0,
  read_count INT DEFAULT 0
);

-- Message receipts
CREATE TABLE message_receipts (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  message_id UUID NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
  user_id UUID NOT NULL REFERENCES users(id),
  receipt_type VARCHAR(50) NOT NULL, -- 'delivered', 'read'
  created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Sessions table for WebSocket tracking
CREATE TABLE sessions (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  session_token_hash VARCHAR(255) NOT NULL,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
  last_activity TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
  expires_at TIMESTAMP WITH TIME ZONE,
  ip_address INET,
  user_agent TEXT,
  is_active BOOLEAN DEFAULT TRUE
);

-- Audit log for compliance (GDPR/HIPAA)
CREATE TABLE audit_logs (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  user_id UUID REFERENCES users(id),
  action VARCHAR(100) NOT NULL,
  resource_type VARCHAR(100),
  resource_id VARCHAR(255),
  changes JSONB,
  ip_address INET,
  user_agent TEXT,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Rate limiting
CREATE TABLE rate_limits (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  identifier VARCHAR(255) NOT NULL, -- IP or user_id
  endpoint VARCHAR(255) NOT NULL,
  request_count INT DEFAULT 1,
  reset_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP + INTERVAL '1 minute'
);

-- Create indices for performance
CREATE INDEX idx_users_public_key_hash ON users(public_key_hash);
CREATE INDEX idx_users_created_at ON users(created_at);
CREATE INDEX idx_messages_sender_id ON messages(sender_id);
CREATE INDEX idx_messages_recipient_id ON messages(recipient_id);
CREATE INDEX idx_messages_created_at ON messages(created_at);
CREATE INDEX idx_messages_expires_at ON messages(ttl_expires_at);
CREATE INDEX idx_prekey_bundles_user_id ON prekey_bundles(user_id);
CREATE INDEX idx_sessions_user_id ON sessions(user_id);
CREATE INDEX idx_sessions_token_hash ON sessions(session_token_hash);
CREATE INDEX idx_audit_logs_user_id ON audit_logs(user_id);
CREATE INDEX idx_audit_logs_created_at ON audit_logs(created_at);
CREATE INDEX idx_rate_limits_identifier ON rate_limits(identifier);
CREATE INDEX idx_rate_limits_reset_at ON rate_limits(reset_at);

-- Enable row-level security
ALTER TABLE users ENABLE ROW LEVEL SECURITY;
ALTER TABLE messages ENABLE ROW LEVEL SECURITY;
ALTER TABLE sessions ENABLE ROW LEVEL SECURITY;
ALTER TABLE audit_logs ENABLE ROW LEVEL SECURITY;

-- RLS Policy: Users can only see their own data
CREATE POLICY user_self_access ON users
  USING (id = uuid_nil() OR true)  -- TODO: implement user context
  WITH CHECK (id = uuid_nil() OR true);

-- Create functions for updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = CURRENT_TIMESTAMP;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users
  FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
