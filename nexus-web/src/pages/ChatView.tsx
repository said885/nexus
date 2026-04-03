import React, { useState, useEffect, useRef } from 'react';
import { Send, Phone, Video, MoreVertical, File, Smile } from 'lucide-react';

interface Message {
  id: string;
  sender: string;
  content: string;
  timestamp: number;
  encrypted: boolean;
  isOutgoing: boolean;
  status: 'sending' | 'sent' | 'delivered' | 'read';
}

interface ChatViewProps {
  participantName: string;
  messages: Message[];
  onSendMessage: (content: string) => Promise<void>;
  onCallStart: (type: 'audio' | 'video') => void;
  loading?: boolean;
}

export const ChatView: React.FC<ChatViewProps> = ({
  participantName,
  messages,
  onSendMessage,
  onCallStart,
  loading = false,
}) => {
  const [inputText, setInputText] = useState('');
  const [sending, setSending] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  const handleSendMessage = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!inputText.trim() || sending) return;

    const text = inputText.trim();
    setInputText('');
    setSending(true);

    try {
      await onSendMessage(text);
    } finally {
      setSending(false);
    }
  };

  const formatTime = (timestamp: number) => {
    return new Date(timestamp).toLocaleTimeString('en-US', {
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'sending':
        return '⏱️';
      case 'sent':
        return '✓';
      case 'delivered':
        return '✓✓';
      case 'read':
        return '✓✓';
      default:
        return '';
    }
  };

  return (
    <div className="flex flex-col h-full bg-gray-900">
      {/* Header */}
      <div className="border-b border-gray-800 p-4 flex items-center justify-between">
        <div>
          <h2 className="text-xl font-bold text-white">{participantName}</h2>
          <p className="text-sm text-gray-400">End-to-end encrypted</p>
        </div>
        <div className="flex gap-2">
          <button
            onClick={() => onCallStart('audio')}
            className="p-2 hover:bg-gray-800 rounded-lg transition text-gray-400 hover:text-white"
            title="Start audio call"
          >
            <Phone className="w-5 h-5" />
          </button>
          <button
            onClick={() => onCallStart('video')}
            className="p-2 hover:bg-gray-800 rounded-lg transition text-gray-400 hover:text-white"
            title="Start video call"
          >
            <Video className="w-5 h-5" />
          </button>
          <button className="p-2 hover:bg-gray-800 rounded-lg transition text-gray-400 hover:text-white">
            <MoreVertical className="w-5 h-5" />
          </button>
        </div>
      </div>

      {/* Messages */}
      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        {messages.length === 0 ? (
          <div className="h-full flex items-center justify-center">
            <p className="text-gray-500">No messages yet. Start a conversation!</p>
          </div>
        ) : (
          messages.map((msg) => (
            <div
              key={msg.id}
              className={`flex ${msg.isOutgoing ? 'justify-end' : 'justify-start'}`}
            >
              <div
                className={`max-w-xs px-4 py-2 rounded-lg ${
                  msg.isOutgoing
                    ? 'bg-purple-600 text-white'
                    : 'bg-gray-800 text-gray-100'
                }`}
              >
                <p className="break-words">{msg.content}</p>
                <div className="flex items-center justify-between gap-2 mt-1">
                  <p className="text-xs opacity-70">{formatTime(msg.timestamp)}</p>
                  {msg.isOutgoing && (
                    <p className="text-xs opacity-70">{getStatusIcon(msg.status)}</p>
                  )}
                </div>
              </div>
            </div>
          ))
        )}
        <div ref={messagesEndRef} />
      </div>

      {/* Input */}
      <div className="border-t border-gray-800 p-4">
        <form onSubmit={handleSendMessage} className="flex gap-2">
          <button
            type="button"
            className="p-2 hover:bg-gray-800 rounded-lg transition text-gray-400 hover:text-white"
          >
            <File className="w-5 h-5" />
          </button>
          <button
            type="button"
            className="p-2 hover:bg-gray-800 rounded-lg transition text-gray-400 hover:text-white"
          >
            <Smile className="w-5 h-5" />
          </button>
          <input
            type="text"
            value={inputText}
            onChange={(e) => setInputText(e.target.value)}
            placeholder="Type a message..."
            disabled={sending || loading}
            className="flex-1 bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-white focus:outline-none focus:border-purple-500 disabled:opacity-50"
          />
          <button
            type="submit"
            disabled={!inputText.trim() || sending || loading}
            className="bg-purple-600 hover:bg-purple-700 disabled:opacity-50 text-white px-4 py-2 rounded-lg transition flex items-center gap-2"
          >
            <Send className="w-4 h-4" />
          </button>
        </form>
      </div>
    </div>
  );
};
