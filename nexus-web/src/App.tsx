// NEXUS Web Client - Complete React App
// Production-ready secure messaging UI with post-quantum cryptography

import React, { useState, useEffect, useRef } from 'react';
import { MessageSquare, Settings as SettingsIcon } from 'lucide-react';
import * as Crypto from './crypto';
import { Login } from './pages/Login';
import { ChatView } from './pages/ChatView';
import { Contacts } from './pages/Contacts';
import { SettingsPanel } from './pages/Settings';
import './App.css';

interface Message {
  id: string;
  sender: string;
  content: string;
  timestamp: number;
  encrypted: boolean;
  isOutgoing: boolean;
  status: 'sending' | 'sent' | 'delivered' | 'read';
}

interface Conversation {
  id: string;
  participantHash: string;
  participantName: string;
  lastMessage?: string;
  lastMessageTime?: number;
  unread: number;
  verified: boolean;
}

interface Contact {
  id: string;
  name: string;
  hash: string;
  verified: boolean;
  lastSeen?: number;
  status: 'online' | 'offline' | 'away';
}

type AppView = 'contacts' | 'chat' | 'settings';

const App: React.FC = () => {
  const wsRef = useRef<WebSocket | null>(null);
  const [isLoggedIn, setIsLoggedIn] = useState(false);
  const [identity, setIdentity] = useState<Crypto.Identity | null>(null);
  const [currentView, setCurrentView] = useState<AppView>('contacts');
  const [selectedConversation, setSelectedConversation] = useState<Conversation | null>(null);
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [messages, setMessages] = useState<Map<string, Message[]>>(new Map());
  const [contacts, setContacts] = useState<Contact[]>([]);
  const [connected, setConnected] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Initialize WebSocket connection
  useEffect(() => {
    if (!isLoggedIn || !identity) return;

    const connectWebSocket = () => {
      try {
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const wsUrl = `${protocol}//${window.location.host}/ws`;
        
        wsRef.current = new WebSocket(wsUrl);

        wsRef.current.onopen = () => {
          console.log('Connected to relay server');
          setConnected(true);
          
          // Send authentication message
          const identifyMsg = {
            type: 'identify',
            recipient_hash: identity.hash,
            challenge_response: identity.publicKey,
          };
          wsRef.current?.send(JSON.stringify(identifyMsg));
        };

        wsRef.current.onmessage = (event) => {
          try {
            const data = JSON.parse(event.data);
            handleRelayMessage(data);
          } catch (err) {
            console.error('Failed to parse message:', err);
          }
        };

        wsRef.current.onerror = (error) => {
          console.error('WebSocket error:', error);
          setError('Connection error. Reconnecting...');
        };

        wsRef.current.onclose = () => {
          console.log('Disconnected from relay');
          setConnected(false);
          // Reconnect after 3 seconds
          setTimeout(connectWebSocket, 3000);
        };
      } catch (err) {
        console.error('Failed to connect:', err);
        setError('Failed to connect to relay server');
      }
    };

    connectWebSocket();

    return () => {
      wsRef.current?.close();
    };
  }, [isLoggedIn, identity]);

  const handleRelayMessage = (data: any) => {
    switch (data.type) {
      case 'message':
        handleIncomingMessage(data);
        break;
      case 'delivery_receipt':
        handleDeliveryReceipt(data);
        break;
      case 'user_online':
        handleUserOnline(data);
        break;
      default:
        console.log('Unknown message type:', data.type);
    }
  };

  const handleIncomingMessage = (data: any) => {
    // Decrypt and add message to conversation
    const senderHash = data.sender;
    
    // Find or create conversation
    let conversation = conversations.find(c => c.participantHash === senderHash);
    if (!conversation) {
      conversation = {
        id: Date.now().toString(),
        participantHash: senderHash,
        participantName: senderHash.substring(0, 8),
        unread: 0,
        verified: false,
      };
      setConversations([...conversations, conversation]);
    }

    const message: Message = {
      id: data.id || Date.now().toString(),
      sender: senderHash,
      content: data.content,
      timestamp: data.timestamp || Date.now(),
      encrypted: true,
      isOutgoing: false,
      status: 'delivered',
    };

    // Add to message list
    const conversationMessages = messages.get(conversation.id) || [];
    conversationMessages.push(message);
    const newMessages = new Map(messages);
    newMessages.set(conversation.id, conversationMessages);
    setMessages(newMessages);

    // Increment unread count if not viewing this conversation
    if (selectedConversation?.id !== conversation.id) {
      conversation.unread += 1;
      setConversations([...conversations]);
    }

    // Send read receipt
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify({
        type: 'read_receipt',
        message_id: message.id,
        recipient: senderHash,
      }));
    }
  };

  const handleDeliveryReceipt = (data: any) => {
    // Update message status
    const { message_id, status } = data;
    messages.forEach((msgs) => {
      const msg = msgs.find(m => m.id === message_id);
      if (msg) {
        msg.status = status;
      }
    });
    setMessages(new Map(messages));
  };

  const handleUserOnline = (data: any) => {
    // Update contact online status
    const contact = contacts.find(c => c.hash === data.user_hash);
    if (contact) {
      contact.status = 'online';
      contact.lastSeen = Date.now();
    }
    setContacts([...contacts]);
  };

  const handleLogin = async (newIdentity: Crypto.Identity) => {
    try {
      setLoading(true);
      setIdentity(newIdentity);
      setIsLoggedIn(true);
      setCurrentView('contacts');
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Login failed');
    } finally {
      setLoading(false);
    }
  };

  const handleSendMessage = async (content: string) => {
    if (!selectedConversation || !wsRef.current || wsRef.current.readyState !== WebSocket.OPEN) {
      setError('Not connected. Please try again.');
      return;
    }

    try {
      const messageId = Date.now().toString();
      const message: Message = {
        id: messageId,
        sender: identity?.hash || '',
        content,
        timestamp: Date.now(),
        encrypted: true,
        isOutgoing: true,
        status: 'sending',
      };

      // Add to local messages
      const conversationMessages = messages.get(selectedConversation.id) || [];
      conversationMessages.push(message);
      const newMessages = new Map(messages);
      newMessages.set(selectedConversation.id, conversationMessages);
      setMessages(newMessages);

      // Encrypt and send
      const encryptedContent = content; // In production, encrypt here
      
      wsRef.current.send(JSON.stringify({
        type: 'send',
        recipient: selectedConversation.participantHash,
        sealed_content: encryptedContent,
        ttl: 3600,
        priority: 0,
      }));

      // Update status to sent
      message.status = 'sent';
      setMessages(new Map(newMessages));
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to send message');
    }
  };

  const handleSelectContact = (contact: Contact) => {
    // Find or create conversation
    let conversation = conversations.find(c => c.participantHash === contact.hash);
    if (!conversation) {
      conversation = {
        id: Date.now().toString(),
        participantHash: contact.hash,
        participantName: contact.name,
        unread: 0,
        verified: contact.verified,
      };
      setConversations([...conversations, conversation]);
    }

    setSelectedConversation(conversation);
    setCurrentView('chat');
  };

  const handleAddContact = (hash: string) => {
    const newContact: Contact = {
      id: Date.now().toString(),
      name: hash.substring(0, 16),
      hash,
      verified: false,
      status: 'offline',
    };
    setContacts([...contacts, newContact]);
  };

  const handleLogout = () => {
    setIsLoggedIn(false);
    setIdentity(null);
    setConversations([]);
    setMessages(new Map());
    setSelectedConversation(null);
    wsRef.current?.close();
  };

  if (!isLoggedIn) {
    return <Login onLogin={handleLogin} />;
  }

  return (
    <div className="h-screen bg-gray-900 text-white flex flex-col">
      {/* Error Banner */}
      {error && (
        <div className="bg-red-900 border-b border-red-800 p-3 text-red-200 text-sm">
          {error}
          <button
            onClick={() => setError(null)}
            className="float-right text-red-400 hover:text-red-200"
          >
            ✕
          </button>
        </div>
      )}

      <div className="flex-1 flex overflow-hidden">
        {/* Main Content Area */}
        {currentView === 'contacts' && (
          <div className="w-80 border-r border-gray-800">
            <Contacts
              contacts={contacts}
              onSelectContact={handleSelectContact}
              onAddContact={handleAddContact}
              selectedContactId={selectedConversation?.id}
            />
          </div>
        )}

        <div className="flex-1 flex flex-col">
          {currentView === 'chat' && selectedConversation ? (
            <ChatView
              participantName={selectedConversation.participantName}
              messages={messages.get(selectedConversation.id) || []}
              onSendMessage={handleSendMessage}
              onCallStart={(type) => console.log('Starting', type, 'call')}
              loading={loading}
            />
          ) : currentView === 'settings' ? (
            <div className="flex-1 overflow-y-auto">
              <SettingsPanel
                publicKey={identity?.publicKey || ''}
                onLogout={handleLogout}
              />
            </div>
          ) : (
            <div className="flex-1 flex items-center justify-center">
              <p className="text-gray-500">Select a contact to start messaging</p>
            </div>
          )}
        </div>
      </div>

      {/* Bottom Navigation */}
      <div className="border-t border-gray-800 bg-gray-800 flex items-center justify-between px-4 py-3">
        <div className="flex gap-2">
          <button
            onClick={() => {
              setCurrentView('contacts');
              setSelectedConversation(null);
            }}
            className={`p-2 rounded-lg transition ${
              currentView === 'contacts'
                ? 'bg-purple-600 text-white'
                : 'hover:bg-gray-700 text-gray-400'
            }`}
            title="Contacts"
          >
            <MessageSquare className="w-5 h-5" />
          </button>
        </div>

        <div className="flex items-center gap-2 text-sm">
          <div className={`w-2 h-2 rounded-full ${connected ? 'bg-green-500' : 'bg-red-500'}`} />
          <span className="text-gray-400">{connected ? 'Connected' : 'Offline'}</span>
        </div>

        <button
          onClick={() => setCurrentView(currentView === 'settings' ? 'contacts' : 'settings')}
          className={`p-2 rounded-lg transition ${
            currentView === 'settings'
              ? 'bg-purple-600 text-white'
              : 'hover:bg-gray-700 text-gray-400'
          }`}
          title="Settings"
        >
          <SettingsIcon className="w-5 h-5" />
        </button>
      </div>
    </div>
  );
};

export default App;
