// NEXUS Web Client - Group Chat Component
import React, { useState, useEffect, useCallback } from 'react';

interface GroupMember {
  id: string;
  name: string;
  isAdmin: boolean;
  isOnline: boolean;
}

interface Group {
  id: string;
  name: string;
  description?: string;
  owner: string;
  members: GroupMember[];
  createdAt: number;
}

interface GroupsProps {
  currentUserHash: string;
  onGroupSelect: (groupId: string) => void;
  onCreateGroup: (name: string, description?: string) => void;
  onLeaveGroup: (groupId: string) => void;
  onAddMember: (groupId: string, memberHash: string) => void;
  onRemoveMember: (groupId: string, memberHash: string) => void;
}

// Groups list component
export const GroupsList: React.FC<GroupsProps> = ({
  currentUserHash,
  onGroupSelect,
  onCreateGroup,
}) => {
  const [groups, setGroups] = useState<Group[]>([]);
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [newGroupName, setNewGroupName] = useState('');
  const [newGroupDescription, setNewGroupDescription] = useState('');
  const [selectedGroup, setSelectedGroup] = useState<Group | null>(null);

  // Load groups (in production, fetch from server)
  useEffect(() => {
    // Sample data
    setGroups([
      {
        id: 'group-1',
        name: 'Development Team',
        description: 'Main development discussion',
        owner: 'alice-hash',
        members: [
          { id: 'alice-hash', name: 'Alice', isAdmin: true, isOnline: true },
          { id: 'bob-hash', name: 'Bob', isAdmin: false, isOnline: true },
          { id: 'charlie-hash', name: 'Charlie', isAdmin: false, isOnline: false },
        ],
        createdAt: Date.now(),
      },
      {
        id: 'group-2',
        name: 'Security Team',
        description: 'Security discussions',
        owner: 'bob-hash',
        members: [
          { id: 'bob-hash', name: 'Bob', isAdmin: true, isOnline: true },
          { id: 'dave-hash', name: 'Dave', isAdmin: false, isOnline: false },
        ],
        createdAt: Date.now(),
      },
    ]);
  }, []);

  const handleCreateGroup = useCallback(() => {
    if (newGroupName.trim()) {
      onCreateGroup(newGroupName, newGroupDescription || undefined);
      setNewGroupName('');
      setNewGroupDescription('');
      setShowCreateModal(false);
    }
  }, [newGroupName, newGroupDescription, onCreateGroup]);

  const handleGroupClick = useCallback((group: Group) => {
    setSelectedGroup(group);
    onGroupSelect(group.id);
  }, [onGroupSelect]);

  return (
    <div style={styles.container}>
      {/* Header */}
      <div style={styles.header}>
        <h2 style={styles.title}>Groups</h2>
        <button
          style={styles.createButton}
          onClick={() => setShowCreateModal(true)}
        >
          + New Group
        </button>
      </div>

      {/* Groups list */}
      <div style={styles.groupsList}>
        {groups.length === 0 ? (
          <div style={styles.emptyState}>
            <div style={styles.emptyIcon}>👥</div>
            <h3 style={styles.emptyTitle}>No Groups Yet</h3>
            <p style={styles.emptyText}>
              Create a group to start chatting with multiple people securely
            </p>
            <button
              style={styles.createButton}
              onClick={() => setShowCreateModal(true)}
            >
              Create Group
            </button>
          </div>
        ) : (
          groups.map(group => (
            <div
              key={group.id}
              style={{
                ...styles.groupItem,
                backgroundColor: selectedGroup?.id === group.id ? '#2a2a4a' : 'transparent',
              }}
              onClick={() => handleGroupClick(group)}
            >
              <div style={styles.groupAvatar}>
                👥
              </div>
              <div style={styles.groupInfo}>
                <div style={styles.groupName}>
                  {group.name}
                  {group.owner === currentUserHash && (
                    <span style={styles.ownerBadge}>Owner</span>
                  )}
                </div>
                <div style={styles.groupMeta}>
                  {group.members.length} members
                  {group.description && ` • ${group.description}`}
                </div>
              </div>
              <div style={styles.groupArrow}>›</div>
            </div>
          ))
        )}
      </div>

      {/* Create group modal */}
      {showCreateModal && (
        <div style={styles.modalOverlay}>
          <div style={styles.modal}>
            <div style={styles.modalHeader}>
              <h3 style={styles.modalTitle}>Create New Group</h3>
              <button
                style={styles.closeButton}
                onClick={() => setShowCreateModal(false)}
              >
                ×
              </button>
            </div>

            <div style={styles.modalBody}>
              <div style={styles.formGroup}>
                <label style={styles.label}>Group Name</label>
                <input
                  style={styles.input}
                  type="text"
                  value={newGroupName}
                  onChange={(e) => setNewGroupName(e.target.value)}
                  placeholder="Enter group name"
                  autoFocus
                />
              </div>

              <div style={styles.formGroup}>
                <label style={styles.label}>Description (optional)</label>
                <textarea
                  style={{...styles.input, minHeight: '80px', resize: 'vertical'}}
                  value={newGroupDescription}
                  onChange={(e) => setNewGroupDescription(e.target.value)}
                  placeholder="Enter group description"
                />
              </div>
            </div>

            <div style={styles.modalFooter}>
              <button
                style={styles.cancelButton}
                onClick={() => setShowCreateModal(false)}
              >
                Cancel
              </button>
              <button
                style={{
                  ...styles.createButton,
                  opacity: newGroupName.trim() ? 1 : 0.5,
                }}
                onClick={handleCreateGroup}
                disabled={!newGroupName.trim()}
              >
                Create Group
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

// Group detail component
interface GroupDetailProps {
  group: Group;
  currentUserHash: string;
  onBack: () => void;
  onAddMember: (memberHash: string) => void;
  onRemoveMember: (memberHash: string) => void;
  onLeaveGroup: () => void;
}

export const GroupDetail: React.FC<GroupDetailProps> = ({
  group,
  currentUserHash,
  onBack,
  onAddMember,
  onRemoveMember,
  onLeaveGroup,
}) => {
  const [showAddMember, setShowAddMember] = useState(false);
  const [newMemberHash, setNewMemberHash] = useState('');
  const [showLeaveConfirm, setShowLeaveConfirm] = useState(false);

  const isOwner = group.owner === currentUserHash;

  const handleAddMember = useCallback(() => {
    if (newMemberHash.length === 64) {
      onAddMember(newMemberHash);
      setNewMemberHash('');
      setShowAddMember(false);
    }
  }, [newMemberHash, onAddMember]);

  return (
    <div style={styles.container}>
      {/* Header */}
      <div style={styles.header}>
        <button style={styles.backButton} onClick={onBack}>
          ← Back
        </button>
        <h2 style={styles.title}>Group Info</h2>
        <div style={{ width: '60px' }} />
      </div>

      {/* Group info */}
      <div style={styles.groupInfoCard}>
        <div style={styles.groupAvatarLarge}>👥</div>
        <h3 style={styles.groupNameLarge}>{group.name}</h3>
        {group.description && (
          <p style={styles.groupDescription}>{group.description}</p>
        )}
        <div style={styles.groupStats}>
          <span>{group.members.length} members</span>
          <span>•</span>
          <span>Created {new Date(group.createdAt).toLocaleDateString()}</span>
        </div>
      </div>

      {/* Members section */}
      <div style={styles.section}>
        <div style={styles.sectionHeader}>
          <h3 style={styles.sectionTitle}>Members</h3>
          <button
            style={styles.addButton}
            onClick={() => setShowAddMember(true)}
          >
            + Add
          </button>
        </div>

        <div style={styles.membersList}>
          {group.members.map(member => (
            <div key={member.id} style={styles.memberItem}>
              <div style={styles.memberAvatar}>
                <span style={styles.memberInitial}>
                  {member.name.charAt(0).toUpperCase()}
                </span>
                {member.isOnline && (
                  <div style={styles.onlineIndicator} />
                )}
              </div>

              <div style={styles.memberInfo}>
                <div style={styles.memberName}>
                  {member.name}
                  {member.isAdmin && (
                    <span style={styles.adminBadge}>Admin</span>
                  )}
                </div>
                <div style={styles.memberHash}>
                  {member.id.slice(0, 16)}...
                </div>
              </div>

              {isOwner && !member.isAdmin && member.id !== currentUserHash && (
                <button
                  style={styles.removeButton}
                  onClick={() => onRemoveMember(member.id)}
                >
                  Remove
                </button>
              )}
            </div>
          ))}
        </div>
      </div>

      {/* Actions */}
      <div style={styles.actions}>
        <button
          style={styles.leaveButton}
          onClick={() => setShowLeaveConfirm(true)}
        >
          Leave Group
        </button>
      </div>

      {/* Add member modal */}
      {showAddMember && (
        <div style={styles.modalOverlay}>
          <div style={styles.modal}>
            <div style={styles.modalHeader}>
              <h3 style={styles.modalTitle}>Add Member</h3>
              <button
                style={styles.closeButton}
                onClick={() => setShowAddMember(false)}
              >
                ×
              </button>
            </div>

            <div style={styles.modalBody}>
              <div style={styles.formGroup}>
                <label style={styles.label}>Member Identity Hash</label>
                <input
                  style={styles.input}
                  type="text"
                  value={newMemberHash}
                  onChange={(e) => setNewMemberHash(e.target.value)}
                  placeholder="64-character hex string"
                  maxLength={64}
                  autoFocus
                />
                {newMemberHash.length > 0 && newMemberHash.length !== 64 && (
                  <div style={styles.errorText}>
                    Identity hash must be 64 characters
                  </div>
                )}
              </div>
            </div>

            <div style={styles.modalFooter}>
              <button
                style={styles.cancelButton}
                onClick={() => setShowAddMember(false)}
              >
                Cancel
              </button>
              <button
                style={{
                  ...styles.createButton,
                  opacity: newMemberHash.length === 64 ? 1 : 0.5,
                }}
                onClick={handleAddMember}
                disabled={newMemberHash.length !== 64}
              >
                Add Member
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Leave confirmation */}
      {showLeaveConfirm && (
        <div style={styles.modalOverlay}>
          <div style={styles.modal}>
            <div style={styles.modalHeader}>
              <h3 style={styles.modalTitle}>Leave Group</h3>
              <button
                style={styles.closeButton}
                onClick={() => setShowLeaveConfirm(false)}
              >
                ×
              </button>
            </div>

            <div style={styles.modalBody}>
              <p>Are you sure you want to leave this group?</p>
            </div>

            <div style={styles.modalFooter}>
              <button
                style={styles.cancelButton}
                onClick={() => setShowLeaveConfirm(false)}
              >
                Cancel
              </button>
              <button
                style={styles.leaveButton}
                onClick={onLeaveGroup}
              >
                Leave Group
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

// Styles
const styles: { [key: string]: React.CSSProperties } = {
  container: {
    display: 'flex',
    flexDirection: 'column',
    height: '100%',
    backgroundColor: '#0f0f1a',
  },
  header: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    padding: '1rem 1.5rem',
    borderBottom: '1px solid #2a2a4a',
  },
  title: {
    margin: 0,
    fontSize: '1.25rem',
    fontWeight: 600,
  },
  createButton: {
    padding: '0.5rem 1rem',
    backgroundColor: '#6C63FF',
    color: 'white',
    border: 'none',
    borderRadius: '6px',
    cursor: 'pointer',
    fontSize: '0.875rem',
    fontWeight: 500,
  },
  backButton: {
    padding: '0.5rem',
    backgroundColor: 'transparent',
    color: '#6C63FF',
    border: 'none',
    cursor: 'pointer',
    fontSize: '1rem',
  },
  groupsList: {
    flex: 1,
    overflowY: 'auto',
  },
  emptyState: {
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    justifyContent: 'center',
    height: '100%',
    padding: '2rem',
  },
  emptyIcon: {
    fontSize: '4rem',
    marginBottom: '1rem',
  },
  emptyTitle: {
    margin: '0 0 0.5rem',
    fontSize: '1.5rem',
    fontWeight: 600,
  },
  emptyText: {
    margin: '0 0 1.5rem',
    color: '#888',
    textAlign: 'center',
  },
  groupItem: {
    display: 'flex',
    alignItems: 'center',
    padding: '1rem 1.5rem',
    borderBottom: '1px solid #2a2a4a',
    cursor: 'pointer',
  },
  groupAvatar: {
    width: '50px',
    height: '50px',
    borderRadius: '50%',
    backgroundColor: '#2a2a4a',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    fontSize: '1.5rem',
    marginRight: '1rem',
  },
  groupInfo: {
    flex: 1,
  },
  groupName: {
    fontWeight: 600,
    display: 'flex',
    alignItems: 'center',
    gap: '0.5rem',
  },
  ownerBadge: {
    fontSize: '0.75rem',
    backgroundColor: '#6C63FF',
    color: 'white',
    padding: '0.125rem 0.5rem',
    borderRadius: '4px',
  },
  groupMeta: {
    fontSize: '0.875rem',
    color: '#888',
    marginTop: '0.25rem',
  },
  groupArrow: {
    fontSize: '1.5rem',
    color: '#666',
  },
  groupInfoCard: {
    padding: '2rem',
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    borderBottom: '1px solid #2a2a4a',
  },
  groupAvatarLarge: {
    width: '80px',
    height: '80px',
    borderRadius: '50%',
    backgroundColor: '#2a2a4a',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    fontSize: '2.5rem',
    marginBottom: '1rem',
  },
  groupNameLarge: {
    margin: '0 0 0.5rem',
    fontSize: '1.5rem',
    fontWeight: 600,
  },
  groupDescription: {
    margin: '0 0 0.5rem',
    color: '#888',
    textAlign: 'center',
  },
  groupStats: {
    display: 'flex',
    gap: '0.5rem',
    fontSize: '0.875rem',
    color: '#666',
  },
  section: {
    padding: '1rem 1.5rem',
  },
  sectionHeader: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: '1rem',
  },
  sectionTitle: {
    margin: 0,
    fontSize: '1rem',
    fontWeight: 600,
  },
  addButton: {
    padding: '0.5rem 1rem',
    backgroundColor: '#2a2a4a',
    color: '#6C63FF',
    border: 'none',
    borderRadius: '6px',
    cursor: 'pointer',
    fontSize: '0.875rem',
  },
  membersList: {
    display: 'flex',
    flexDirection: 'column',
    gap: '0.5rem',
  },
  memberItem: {
    display: 'flex',
    alignItems: 'center',
    padding: '0.75rem',
    backgroundColor: '#1a1a2e',
    borderRadius: '8px',
  },
  memberAvatar: {
    position: 'relative',
    width: '40px',
    height: '40px',
    borderRadius: '50%',
    backgroundColor: '#6C63FF',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    marginRight: '0.75rem',
  },
  memberInitial: {
    color: 'white',
    fontWeight: 600,
  },
  onlineIndicator: {
    position: 'absolute',
    bottom: 0,
    right: 0,
    width: '12px',
    height: '12px',
    borderRadius: '50%',
    backgroundColor: '#4CAF50',
    border: '2px solid #0f0f1a',
  },
  memberInfo: {
    flex: 1,
  },
  memberName: {
    fontWeight: 500,
    display: 'flex',
    alignItems: 'center',
    gap: '0.5rem',
  },
  adminBadge: {
    fontSize: '0.625rem',
    backgroundColor: '#2a4a2a',
    color: '#4CAF50',
    padding: '0.125rem 0.5rem',
    borderRadius: '4px',
  },
  memberHash: {
    fontSize: '0.75rem',
    color: '#666',
  },
  removeButton: {
    padding: '0.5rem',
    backgroundColor: 'transparent',
    color: '#f44336',
    border: 'none',
    cursor: 'pointer',
    fontSize: '0.875rem',
  },
  actions: {
    padding: '1rem 1.5rem',
    marginTop: 'auto',
  },
  leaveButton: {
    width: '100%',
    padding: '0.75rem',
    backgroundColor: 'transparent',
    color: '#f44336',
    border: '1px solid #f44336',
    borderRadius: '8px',
    cursor: 'pointer',
    fontSize: '0.875rem',
    fontWeight: 500,
  },
  modalOverlay: {
    position: 'fixed',
    top: 0,
    left: 0,
    right: 0,
    bottom: 0,
    backgroundColor: 'rgba(0,0,0,0.8)',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    zIndex: 1000,
  },
  modal: {
    backgroundColor: '#1a1a2e',
    borderRadius: '12px',
    width: '100%',
    maxWidth: '400px',
    margin: '1rem',
  },
  modalHeader: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    padding: '1rem 1.5rem',
    borderBottom: '1px solid #2a2a4a',
  },
  modalTitle: {
    margin: 0,
    fontSize: '1.125rem',
    fontWeight: 600,
  },
  closeButton: {
    background: 'none',
    border: 'none',
    color: '#888',
    fontSize: '1.5rem',
    cursor: 'pointer',
    padding: 0,
    lineHeight: 1,
  },
  modalBody: {
    padding: '1.5rem',
  },
  modalFooter: {
    display: 'flex',
    justifyContent: 'flex-end',
    gap: '0.5rem',
    padding: '1rem 1.5rem',
    borderTop: '1px solid #2a2a4a',
  },
  formGroup: {
    marginBottom: '1rem',
  },
  label: {
    display: 'block',
    marginBottom: '0.5rem',
    fontSize: '0.875rem',
    fontWeight: 500,
  },
  input: {
    width: '100%',
    padding: '0.75rem',
    backgroundColor: '#2a2a4a',
    border: '1px solid #3a3a5a',
    borderRadius: '6px',
    color: '#e0e0e0',
    fontSize: '0.875rem',
    boxSizing: 'border-box',
  },
  cancelButton: {
    padding: '0.5rem 1rem',
    backgroundColor: '#2a2a4a',
    color: '#e0e0e0',
    border: 'none',
    borderRadius: '6px',
    cursor: 'pointer',
    fontSize: '0.875rem',
  },
  errorText: {
    color: '#f44336',
    fontSize: '0.75rem',
    marginTop: '0.25rem',
  },
};

export default GroupsList;
