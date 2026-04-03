// NEXUS Desktop - Tauri Frontend
// Modern UI for the NEXUS Messenger Desktop Client

// State management
const state = {
  connected: false,
  identityHash: null,
  conversations: [],
  messages: new Map(),
  currentChat: null,
  groups: [],
  currentGroup: null,
};

// Initialize app
document.addEventListener('DOMContentLoaded', async () => {
  // Check for existing identity
  const hasIdentity = await invoke('has_identity');
  if (hasIdentity) {
    const hash = await invoke('get_identity_hash');
    state.identityHash = hash;
    showMainScreen();
  } else {
    showSetupScreen();
  }

  // Connect to relay
  connectToRelay();
});

// Show setup screen
function showSetupScreen() {
  document.getElementById('app').innerHTML = `
    <div class="setup-screen">
      <div class="setup-container">
        <div class="logo">
          <div class="logo-icon">🔒</div>
          <h1>NEXUS Messenger</h1>
          <p>Post-Quantum Secure Messaging</p>
        </div>

        <div class="setup-form">
          <button id="generate-identity" class="btn btn-primary">
            Generate Identity
          </button>
          <p class="setup-info">
            This will create your post-quantum cryptographic identity
          </p>
        </div>

        <div class="setup-status" id="setup-status"></div>
      </div>
    </div>
  `;

  document.getElementById('generate-identity').addEventListener('click', async () => {
    const status = document.getElementById('setup-status');
    status.innerHTML = '<div class="spinner"></div> Generating identity...';

    try {
      const hash = await invoke('generate_identity');
      state.identityHash = hash;
      status.innerHTML = '<div class="success">✓ Identity generated!</div>';

      setTimeout(() => showMainScreen(), 1000);
    } catch (error) {
      status.innerHTML = `<div class="error">Error: ${error}</div>`;
    }
  });
}

// Show main screen
function showMainScreen() {
  document.getElementById('app').innerHTML = `
    <div class="main-screen">
      <!-- Sidebar -->
      <div class="sidebar">
        <div class="sidebar-header">
          <div class="user-info">
            <div class="user-avatar">🔒</div>
            <div class="user-details">
              <div class="user-name">NEXUS User</div>
              <div class="user-hash">${state.identityHash ? state.identityHash.slice(0, 8) + '...' : 'Unknown'}</div>
            </div>
          </div>
          <div class="connection-status ${state.connected ? 'connected' : 'disconnected'}">
            ${state.connected ? '🟢' : '🔴'}
          </div>
        </div>

        <div class="sidebar-tabs">
          <button class="tab active" data-tab="chats">Chats</button>
          <button class="tab" data-tab="groups">Groups</button>
        </div>

        <div class="conversations-list" id="conversations-list">
          <div class="empty-state">
            <div class="empty-icon">💬</div>
            <p>No conversations yet</p>
            <button class="btn btn-secondary" id="new-chat-btn">Start a Chat</button>
          </div>
        </div>

        <div class="groups-list" id="groups-list" style="display: none;">
          <div class="empty-state">
            <div class="empty-icon">👥</div>
            <p>No groups yet</p>
            <button class="btn btn-secondary" id="new-group-btn">Create Group</button>
          </div>
        </div>
      </div>

      <!-- Chat area -->
      <div class="chat-area" id="chat-area">
        <div class="chat-placeholder">
          <div class="placeholder-icon">💬</div>
          <h2>Select a conversation</h2>
          <p>Choose a chat or group from the sidebar</p>
        </div>
      </div>
    </div>
  `;

  // Setup tab switching
  document.querySelectorAll('.tab').forEach(tab => {
    tab.addEventListener('click', () => {
      document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
      tab.classList.add('active');

      const tabName = tab.dataset.tab;
      document.getElementById('conversations-list').style.display = tabName === 'chats' ? 'block' : 'none';
      document.getElementById('groups-list').style.display = tabName === 'groups' ? 'block' : 'none';
    });
  });

  // New chat button
  document.getElementById('new-chat-btn').addEventListener('click', showNewChatModal);

  // New group button
  document.getElementById('new-group-btn').addEventListener('click', showNewGroupModal);
}

// Show new chat modal
function showNewChatModal() {
  const modal = document.createElement('div');
  modal.className = 'modal-overlay';
  modal.innerHTML = `
    <div class="modal">
      <div class="modal-header">
        <h3>New Conversation</h3>
        <button class="close-btn" onclick="this.closest('.modal-overlay').remove()">×</button>
      </div>
      <div class="modal-body">
        <div class="form-group">
          <label>Recipient Identity Hash</label>
          <input type="text" id="recipient-hash" placeholder="64-character hex string" maxlength="64">
          <small>Enter the identity hash of the person you want to chat with</small>
        </div>
      </div>
      <div class="modal-footer">
        <button class="btn btn-secondary" onclick="this.closest('.modal-overlay').remove()">Cancel</button>
        <button class="btn btn-primary" id="start-chat-btn">Start Chat</button>
      </div>
    </div>
  `;
  document.body.appendChild(modal);

  document.getElementById('start-chat-btn').addEventListener('click', () => {
    const hash = document.getElementById('recipient-hash').value;
    if (hash.length === 64) {
      createConversation(hash);
      modal.remove();
    }
  });
}

// Show new group modal
function showNewGroupModal() {
  const modal = document.createElement('div');
  modal.className = 'modal-overlay';
  modal.innerHTML = `
    <div class="modal">
      <div class="modal-header">
        <h3>Create Group</h3>
        <button class="close-btn" onclick="this.closest('.modal-overlay').remove()">×</button>
      </div>
      <div class="modal-body">
        <div class="form-group">
          <label>Group Name</label>
          <input type="text" id="group-name" placeholder="Enter group name">
        </div>
        <div class="form-group">
          <label>Description (optional)</label>
          <textarea id="group-description" placeholder="Enter group description"></textarea>
        </div>
      </div>
      <div class="modal-footer">
        <button class="btn btn-secondary" onclick="this.closest('.modal-overlay').remove()">Cancel</button>
        <button class="btn btn-primary" id="create-group-btn">Create Group</button>
      </div>
    </div>
  `;
  document.body.appendChild(modal);

  document.getElementById('create-group-btn').addEventListener('click', async () => {
    const name = document.getElementById('group-name').value;
    const description = document.getElementById('group-description').value;
    if (name.trim()) {
      try {
        await invoke('create_group', { name, description: description || null });
        modal.remove();
        loadGroups();
      } catch (error) {
        alert('Failed to create group: ' + error);
      }
    }
  });
}

// Create conversation
function createConversation(participantHash) {
  const conversation = {
    id: Date.now().toString(),
    participant: participantHash.slice(0, 8) + '...',
    participantHash: participantHash,
    lastMessage: null,
    unread: 0,
    verified: false,
  };

  state.conversations.push(conversation);
  state.messages.set(conversation.id, []);
  renderConversations();
  selectConversation(conversation);
}

// Render conversations
function renderConversations() {
  const list = document.getElementById('conversations-list');

  if (state.conversations.length === 0) {
    list.innerHTML = `
      <div class="empty-state">
        <div class="empty-icon">💬</div>
        <p>No conversations yet</p>
        <button class="btn btn-secondary" id="new-chat-btn">Start a Chat</button>
      </div>
    `;
    return;
  }

  list.innerHTML = state.conversations.map(conv => `
    <div class="conversation-item ${state.currentChat === conv.id ? 'active' : ''}" data-id="${conv.id}">
      <div class="conv-avatar">${conv.verified ? '🔐' : '💬'}</div>
      <div class="conv-info">
        <div class="conv-name">${conv.participant}</div>
        <div class="conv-preview">${conv.lastMessage || 'No messages yet'}</div>
      </div>
      ${conv.unread > 0 ? `<span class="badge">${conv.unread}</span>` : ''}
    </div>
  `).join('');

  // Add click handlers
  list.querySelectorAll('.conversation-item').forEach(item => {
    item.addEventListener('click', () => {
      const conv = state.conversations.find(c => c.id === item.dataset.id);
      if (conv) selectConversation(conv);
    });
  });
}

// Select conversation
function selectConversation(conversation) {
  state.currentChat = conversation.id;
  conversation.unread = 0;
  renderConversations();
  showChatView(conversation);
}

// Show chat view
function showChatView(conversation) {
  const messages = state.messages.get(conversation.id) || [];

  document.getElementById('chat-area').innerHTML = `
    <div class="chat-header">
      <div class="chat-header-info">
        <div class="chat-avatar">${conversation.verified ? '🔐' : '💬'}</div>
        <div class="chat-details">
          <div class="chat-name">${conversation.participant}</div>
          <div class="chat-status">${conversation.verified ? 'Verified' : 'Unverified'}</div>
        </div>
      </div>
      <div class="chat-actions">
        <button class="btn-icon" title="Voice Call">📞</button>
        <button class="btn-icon" title="Video Call">📹</button>
      </div>
    </div>

    <div class="messages-container" id="messages-container">
      ${messages.map(msg => `
        <div class="message ${msg.isOutgoing ? 'outgoing' : 'incoming'}">
          <div class="message-content">${msg.content}</div>
          <div class="message-time">
            ${new Date(msg.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
            ${msg.encrypted ? '🔒' : ''}
          </div>
        </div>
      `).join('')}
    </div>

    <div class="chat-input">
      <input type="text" id="message-input" placeholder="Type a message...">
      <button class="btn btn-primary" id="send-btn">Send</button>
    </div>
  `;

  // Scroll to bottom
  const container = document.getElementById('messages-container');
  container.scrollTop = container.scrollHeight;

  // Send message handler
  document.getElementById('send-btn').addEventListener('click', () => sendMessage(conversation));
  document.getElementById('message-input').addEventListener('keypress', (e) => {
    if (e.key === 'Enter') sendMessage(conversation);
  });
}

// Send message
async function sendMessage(conversation) {
  const input = document.getElementById('message-input');
  const content = input.value.trim();

  if (!content) return;

  const message = {
    id: Date.now().toString(),
    content: content,
    timestamp: Date.now(),
    isOutgoing: true,
    encrypted: true,
  };

  // Add to local messages
  const messages = state.messages.get(conversation.id) || [];
  messages.push(message);
  state.messages.set(conversation.id, messages);

  // Update conversation
  conversation.lastMessage = content;
  renderConversations();

  // Re-render chat
  showChatView(conversation);

  // Send via relay (would use actual encryption)
  try {
    await invoke('send_message', {
      recipientHash: conversation.participantHash,
      content: content,
    });
  } catch (error) {
    console.error('Failed to send message:', error);
  }
}

// Load groups
async function loadGroups() {
  try {
    const groups = await invoke('get_groups');
    state.groups = groups;
    renderGroups();
  } catch (error) {
    console.error('Failed to load groups:', error);
  }
}

// Render groups
function renderGroups() {
  const list = document.getElementById('groups-list');

  if (state.groups.length === 0) {
    list.innerHTML = `
      <div class="empty-state">
        <div class="empty-icon">👥</div>
        <p>No groups yet</p>
        <button class="btn btn-secondary" id="new-group-btn">Create Group</button>
      </div>
    `;
    return;
  }

  list.innerHTML = state.groups.map(group => `
    <div class="group-item ${state.currentGroup === group.id ? 'active' : ''}" data-id="${group.id}">
      <div class="group-avatar">👥</div>
      <div class="group-info">
        <div class="group-name">${group.name}</div>
        <div class="group-meta">${group.members.length} members</div>
      </div>
    </div>
  `).join('');
}

// Connect to relay
async function connectToRelay() {
  try {
    await invoke('connect_to_relay', { url: 'ws://localhost:8443/ws' });
    state.connected = true;
    updateConnectionStatus();
  } catch (error) {
    console.error('Failed to connect:', error);
    state.connected = false;
    updateConnectionStatus();
    setTimeout(connectToRelay, 5000);
  }
}

// Update connection status
function updateConnectionStatus() {
  const status = document.querySelector('.connection-status');
  if (status) {
    status.className = `connection-status ${state.connected ? 'connected' : 'disconnected'}`;
    status.innerHTML = state.connected ? '🟢' : '🔴';
  }
}
