// NEXUS Web Client - State Management
import React, { createContext, useContext, useReducer, useCallback, useEffect } from 'react';

// Types
interface User {
  identityHash: string;
  fingerprint: string;
  displayName?: string;
  status: 'online' | 'away' | 'dnd' | 'invisible' | 'offline';
  statusMessage?: string;
}

interface Message {
  id: string;
  conversationId: string;
  sender: string;
  content: string;
  timestamp: number;
  encrypted: boolean;
  isOutgoing: boolean;
  delivered: boolean;
  read: boolean;
}

interface Conversation {
  id: string;
  participant: string;
  participantHash: string;
  lastMessage?: string;
  lastMessageTime?: number;
  unreadCount: number;
  verified: boolean;
  isTyping: boolean;
}

interface Group {
  id: string;
  name: string;
  description?: string;
  owner: string;
  members: GroupMember[];
  isAdmin: boolean;
}

interface GroupMember {
  id: string;
  name: string;
  isAdmin: boolean;
  isOnline: boolean;
}

interface Call {
  id: string;
  type: 'audio' | 'video';
  status: 'ringing' | 'active' | 'ended';
  initiator: string;
  recipient: string;
  startTime?: number;
  endTime?: number;
}

interface Notification {
  id: string;
  type: 'message' | 'call' | 'group' | 'security';
  title: string;
  body: string;
  timestamp: number;
  read: boolean;
  data?: any;
}

// State
interface NexusState {
  // User
  user: User | null;
  isSetupComplete: boolean;

  // Connection
  isConnected: boolean;
  isConnecting: boolean;

  // Conversations
  conversations: Conversation[];
  currentConversationId: string | null;

  // Messages (keyed by conversation ID)
  messages: Map<string, Message[]>;

  // Groups
  groups: Group[];
  currentGroupId: string | null;

  // Calls
  activeCalls: Call[];
  currentCallId: string | null;

  // Notifications
  notifications: Notification[];
  unreadNotifications: number;

  // UI State
  isLoading: boolean;
  error: string | null;
  showSidebar: boolean;
  theme: 'dark' | 'light';
}

// Actions
type NexusAction =
  | { type: 'SET_USER'; payload: User }
  | { type: 'SETUP_COMPLETE' }
  | { type: 'SET_CONNECTED'; payload: boolean }
  | { type: 'SET_CONNECTING'; payload: boolean }
  | { type: 'ADD_CONVERSATION'; payload: Conversation }
  | { type: 'UPDATE_CONVERSATION'; payload: Partial<Conversation> & { id: string } }
  | { type: 'DELETE_CONVERSATION'; payload: string }
  | { type: 'SET_CURRENT_CONVERSATION'; payload: string | null }
  | { type: 'ADD_MESSAGE'; payload: { conversationId: string; message: Message } }
  | { type: 'UPDATE_MESSAGE'; payload: { conversationId: string; messageId: string; updates: Partial<Message> } }
  | { type: 'MARK_MESSAGES_READ'; payload: string }
  | { type: 'SET_TYPING'; payload: { conversationId: string; isTyping: boolean } }
  | { type: 'ADD_GROUP'; payload: Group }
  | { type: 'UPDATE_GROUP'; payload: Partial<Group> & { id: string } }
  | { type: 'DELETE_GROUP'; payload: string }
  | { type: 'SET_CURRENT_GROUP'; payload: string | null }
  | { type: 'ADD_GROUP_MEMBER'; payload: { groupId: string; member: GroupMember } }
  | { type: 'REMOVE_GROUP_MEMBER'; payload: { groupId: string; memberId: string } }
  | { type: 'ADD_CALL'; payload: Call }
  | { type: 'UPDATE_CALL'; payload: Partial<Call> & { id: string } }
  | { type: 'END_CALL'; payload: string }
  | { type: 'SET_CURRENT_CALL'; payload: string | null }
  | { type: 'ADD_NOTIFICATION'; payload: Notification }
  | { type: 'MARK_NOTIFICATION_READ'; payload: string }
  | { type: 'CLEAR_NOTIFICATIONS' }
  | { type: 'SET_LOADING'; payload: boolean }
  | { type: 'SET_ERROR'; payload: string | null }
  | { type: 'TOGGLE_SIDEBAR' }
  | { type: 'SET_THEME'; payload: 'dark' | 'light' };

// Initial state
const initialState: NexusState = {
  user: null,
  isSetupComplete: false,
  isConnected: false,
  isConnecting: false,
  conversations: [],
  currentConversationId: null,
  messages: new Map(),
  groups: [],
  currentGroupId: null,
  activeCalls: [],
  currentCallId: null,
  notifications: [],
  unreadNotifications: 0,
  isLoading: false,
  error: null,
  showSidebar: true,
  theme: 'dark',
};

// Reducer
function nexusReducer(state: NexusState, action: NexusAction): NexusState {
  switch (action.type) {
    case 'SET_USER':
      return { ...state, user: action.payload };

    case 'SETUP_COMPLETE':
      return { ...state, isSetupComplete: true };

    case 'SET_CONNECTED':
      return { ...state, isConnected: action.payload, isConnecting: false };

    case 'SET_CONNECTING':
      return { ...state, isConnecting: action.payload };

    case 'ADD_CONVERSATION':
      return {
        ...state,
        conversations: [...state.conversations, action.payload],
      };

    case 'UPDATE_CONVERSATION':
      return {
        ...state,
        conversations: state.conversations.map(c =>
          c.id === action.payload.id ? { ...c, ...action.payload } : c
        ),
      };

    case 'DELETE_CONVERSATION':
      return {
        ...state,
        conversations: state.conversations.filter(c => c.id !== action.payload),
        currentConversationId:
          state.currentConversationId === action.payload ? null : state.currentConversationId,
      };

    case 'SET_CURRENT_CONVERSATION':
      return { ...state, currentConversationId: action.payload };

    case 'ADD_MESSAGE': {
      const { conversationId, message } = action.payload;
      const messages = new Map(state.messages);
      const conversationMessages = messages.get(conversationId) || [];
      messages.set(conversationId, [...conversationMessages, message]);

      // Update conversation's last message
      const conversations = state.conversations.map(c =>
        c.id === conversationId
          ? {
              ...c,
              lastMessage: message.content,
              lastMessageTime: message.timestamp,
              unreadCount: message.isOutgoing ? c.unreadCount : c.unreadCount + 1,
            }
          : c
      );

      return { ...state, messages, conversations };
    }

    case 'UPDATE_MESSAGE': {
      const { conversationId, messageId, updates } = action.payload;
      const messages = new Map(state.messages);
      const conversationMessages = messages.get(conversationId) || [];
      messages.set(
        conversationId,
        conversationMessages.map(m => (m.id === messageId ? { ...m, ...updates } : m))
      );
      return { ...state, messages };
    }

    case 'MARK_MESSAGES_READ': {
      const conversationId = action.payload;
      const messages = new Map(state.messages);
      const conversationMessages = messages.get(conversationId) || [];
      messages.set(
        conversationId,
        conversationMessages.map(m => ({ ...m, read: true }))
      );

      const conversations = state.conversations.map(c =>
        c.id === conversationId ? { ...c, unreadCount: 0 } : c
      );

      return { ...state, messages, conversations };
    }

    case 'SET_TYPING':
      return {
        ...state,
        conversations: state.conversations.map(c =>
          c.id === action.payload.conversationId
            ? { ...c, isTyping: action.payload.isTyping }
            : c
        ),
      };

    case 'ADD_GROUP':
      return { ...state, groups: [...state.groups, action.payload] };

    case 'UPDATE_GROUP':
      return {
        ...state,
        groups: state.groups.map(g =>
          g.id === action.payload.id ? { ...g, ...action.payload } : g
        ),
      };

    case 'DELETE_GROUP':
      return {
        ...state,
        groups: state.groups.filter(g => g.id !== action.payload),
        currentGroupId: state.currentGroupId === action.payload ? null : state.currentGroupId,
      };

    case 'SET_CURRENT_GROUP':
      return { ...state, currentGroupId: action.payload };

    case 'ADD_GROUP_MEMBER':
      return {
        ...state,
        groups: state.groups.map(g =>
          g.id === action.payload.groupId
            ? { ...g, members: [...g.members, action.payload.member] }
            : g
        ),
      };

    case 'REMOVE_GROUP_MEMBER':
      return {
        ...state,
        groups: state.groups.map(g =>
          g.id === action.payload.groupId
            ? { ...g, members: g.members.filter(m => m.id !== action.payload.memberId) }
            : g
        ),
      };

    case 'ADD_CALL':
      return { ...state, activeCalls: [...state.activeCalls, action.payload] };

    case 'UPDATE_CALL':
      return {
        ...state,
        activeCalls: state.activeCalls.map(c =>
          c.id === action.payload.id ? { ...c, ...action.payload } : c
        ),
      };

    case 'END_CALL':
      return {
        ...state,
        activeCalls: state.activeCalls.filter(c => c.id !== action.payload),
        currentCallId: state.currentCallId === action.payload ? null : state.currentCallId,
      };

    case 'SET_CURRENT_CALL':
      return { ...state, currentCallId: action.payload };

    case 'ADD_NOTIFICATION':
      return {
        ...state,
        notifications: [action.payload, ...state.notifications],
        unreadNotifications: state.unreadNotifications + 1,
      };

    case 'MARK_NOTIFICATION_READ':
      return {
        ...state,
        notifications: state.notifications.map(n =>
          n.id === action.payload ? { ...n, read: true } : n
        ),
        unreadNotifications: Math.max(0, state.unreadNotifications - 1),
      };

    case 'CLEAR_NOTIFICATIONS':
      return { ...state, notifications: [], unreadNotifications: 0 };

    case 'SET_LOADING':
      return { ...state, isLoading: action.payload };

    case 'SET_ERROR':
      return { ...state, error: action.payload };

    case 'TOGGLE_SIDEBAR':
      return { ...state, showSidebar: !state.showSidebar };

    case 'SET_THEME':
      return { ...state, theme: action.payload };

    default:
      return state;
  }
}

// Context
interface NexusContextValue {
  state: NexusState;
  dispatch: React.Dispatch<NexusAction>;

  // Convenience actions
  setUser: (user: User) => void;
  completeSetup: () => void;
  setConnected: (connected: boolean) => void;
  addConversation: (conversation: Conversation) => void;
  updateConversation: (id: string, updates: Partial<Conversation>) => void;
  deleteConversation: (id: string) => void;
  setCurrentConversation: (id: string | null) => void;
  addMessage: (conversationId: string, message: Message) => void;
  markMessagesRead: (conversationId: string) => void;
  addGroup: (group: Group) => void;
  deleteGroup: (id: string) => void;
  addCall: (call: Call) => void;
  endCall: (callId: string) => void;
  addNotification: (notification: Notification) => void;
  clearNotifications: () => void;
  setError: (error: string | null) => void;
  toggleSidebar: () => void;
  setTheme: (theme: 'dark' | 'light') => void;
}

const NexusContext = createContext<NexusContextValue | null>(null);

// Provider
export function NexusProvider({ children }: { children: React.ReactNode }) {
  const [state, dispatch] = useReducer(nexusReducer, initialState);

  // Load persisted state
  useEffect(() => {
    // Theme preference: OK in localStorage (not sensitive)
    const savedTheme = localStorage.getItem('nexus-theme') as 'dark' | 'light' | null;
    if (savedTheme) {
      dispatch({ type: 'SET_THEME', payload: savedTheme });
    }

    // User identity: use sessionStorage (cleared on tab close, not accessible to other tabs)
    // Never put crypto keys or identity hashes in localStorage — XSS can access them
    const savedUser = sessionStorage.getItem('nexus-user');
    if (savedUser) {
      try {
        const user = JSON.parse(savedUser);
        dispatch({ type: 'SET_USER', payload: user });
        dispatch({ type: 'SETUP_COMPLETE' });
      } catch (e) {
        console.error('Failed to load saved user:', e);
      }
    }
  }, []);

  // Persist theme (non-sensitive)
  useEffect(() => {
    localStorage.setItem('nexus-theme', state.theme);
    document.documentElement.setAttribute('data-theme', state.theme);
  }, [state.theme]);

  // Persist user in sessionStorage only (sensitive: identity hash + fingerprint)
  useEffect(() => {
    if (state.user) {
      sessionStorage.setItem('nexus-user', JSON.stringify(state.user));
    } else {
      sessionStorage.removeItem('nexus-user');
    }
  }, [state.user]);

  // Convenience actions
  const setUser = useCallback((user: User) => {
    dispatch({ type: 'SET_USER', payload: user });
  }, []);

  const completeSetup = useCallback(() => {
    dispatch({ type: 'SETUP_COMPLETE' });
  }, []);

  const setConnected = useCallback((connected: boolean) => {
    dispatch({ type: 'SET_CONNECTED', payload: connected });
  }, []);

  const addConversation = useCallback((conversation: Conversation) => {
    dispatch({ type: 'ADD_CONVERSATION', payload: conversation });
  }, []);

  const updateConversation = useCallback((id: string, updates: Partial<Conversation>) => {
    dispatch({ type: 'UPDATE_CONVERSATION', payload: { id, ...updates } });
  }, []);

  const deleteConversation = useCallback((id: string) => {
    dispatch({ type: 'DELETE_CONVERSATION', payload: id });
  }, []);

  const setCurrentConversation = useCallback((id: string | null) => {
    dispatch({ type: 'SET_CURRENT_CONVERSATION', payload: id });
    if (id) {
      dispatch({ type: 'MARK_MESSAGES_READ', payload: id });
    }
  }, []);

  const addMessage = useCallback((conversationId: string, message: Message) => {
    dispatch({ type: 'ADD_MESSAGE', payload: { conversationId, message } });
  }, []);

  const markMessagesRead = useCallback((conversationId: string) => {
    dispatch({ type: 'MARK_MESSAGES_READ', payload: conversationId });
  }, []);

  const addGroup = useCallback((group: Group) => {
    dispatch({ type: 'ADD_GROUP', payload: group });
  }, []);

  const deleteGroup = useCallback((id: string) => {
    dispatch({ type: 'DELETE_GROUP', payload: id });
  }, []);

  const addCall = useCallback((call: Call) => {
    dispatch({ type: 'ADD_CALL', payload: call });
  }, []);

  const endCall = useCallback((callId: string) => {
    dispatch({ type: 'END_CALL', payload: callId });
  }, []);

  const addNotification = useCallback((notification: Notification) => {
    dispatch({ type: 'ADD_NOTIFICATION', payload: notification });
  }, []);

  const clearNotifications = useCallback(() => {
    dispatch({ type: 'CLEAR_NOTIFICATIONS' });
  }, []);

  const setError = useCallback((error: string | null) => {
    dispatch({ type: 'SET_ERROR', payload: error });
  }, []);

  const toggleSidebar = useCallback(() => {
    dispatch({ type: 'TOGGLE_SIDEBAR' });
  }, []);

  const setTheme = useCallback((theme: 'dark' | 'light') => {
    dispatch({ type: 'SET_THEME', payload: theme });
  }, []);

  const value: NexusContextValue = {
    state,
    dispatch,
    setUser,
    completeSetup,
    setConnected,
    addConversation,
    updateConversation,
    deleteConversation,
    setCurrentConversation,
    addMessage,
    markMessagesRead,
    addGroup,
    deleteGroup,
    addCall,
    endCall,
    addNotification,
    clearNotifications,
    setError,
    toggleSidebar,
    setTheme,
  };

  return React.createElement(NexusContext.Provider, { value }, children);
}

// Hook
export function useNexus() {
  const context = useContext(NexusContext);
  if (!context) {
    throw new Error('useNexus must be used within a NexusProvider');
  }
  return context;
}

// Selectors
export function useUser() {
  return useNexus().state.user;
}

export function useConversations() {
  return useNexus().state.conversations;
}

export function useCurrentConversation() {
  const { state } = useNexus();
  return state.conversations.find(c => c.id === state.currentConversationId) || null;
}

export function useMessages(conversationId: string) {
  return useNexus().state.messages.get(conversationId) || [];
}

export function useGroups() {
  return useNexus().state.groups;
}

export function useActiveCalls() {
  return useNexus().state.activeCalls;
}

export function useNotifications() {
  return useNexus().state.notifications;
}

export function useIsConnected() {
  return useNexus().state.isConnected;
}

export function useTheme() {
  return useNexus().state.theme;
}

export type { User, Message, Conversation, Group, GroupMember, Call, Notification, NexusState };
