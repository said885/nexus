import React, { useState } from 'react';
import { Users, Plus, Search, Phone } from 'lucide-react';

interface Contact {
  id: string;
  name: string;
  hash: string;
  verified: boolean;
  lastSeen?: number;
  status: 'online' | 'offline' | 'away';
}

interface ContactsProps {
  contacts: Contact[];
  onSelectContact: (contact: Contact) => void;
  onAddContact: (hash: string) => void;
  selectedContactId?: string;
}

export const Contacts: React.FC<ContactsProps> = ({
  contacts,
  onSelectContact,
  onAddContact,
  selectedContactId,
}) => {
  const [showAddModal, setShowAddModal] = useState(false);
  const [newContactHash, setNewContactHash] = useState('');
  const [searchTerm, setSearchTerm] = useState('');

  const filtered = contacts.filter(
    (c) => c.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
           c.hash.includes(searchTerm)
  );

  const handleAddContact = () => {
    if (newContactHash.trim()) {
      onAddContact(newContactHash.trim());
      setNewContactHash('');
      setShowAddModal(false);
    }
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'online':
        return 'bg-green-500';
      case 'away':
        return 'bg-yellow-500';
      default:
        return 'bg-gray-500';
    }
  };

  return (
    <div className="w-full h-full bg-gray-900 border-r border-gray-800 flex flex-col">
      {/* Header */}
      <div className="p-4 border-b border-gray-800">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2">
            <Users className="w-5 h-5 text-purple-500" />
            <h2 className="text-lg font-bold text-white">Contacts</h2>
          </div>
          <button
            onClick={() => setShowAddModal(true)}
            className="p-2 hover:bg-gray-800 rounded-lg transition text-gray-400 hover:text-white"
          >
            <Plus className="w-5 h-5" />
          </button>
        </div>

        {/* Search */}
        <div className="relative">
          <Search className="absolute left-3 top-2.5 w-4 h-4 text-gray-500" />
          <input
            type="text"
            placeholder="Search contacts..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="w-full bg-gray-800 border border-gray-700 rounded-lg pl-10 pr-4 py-2 text-white text-sm focus:outline-none focus:border-purple-500"
          />
        </div>
      </div>

      {/* Contacts List */}
      <div className="flex-1 overflow-y-auto">
        {filtered.length === 0 ? (
          <div className="p-4 text-center text-gray-500">
            {contacts.length === 0 ? 'No contacts yet' : 'No matching contacts'}
          </div>
        ) : (
          filtered.map((contact) => (
            <button
              key={contact.id}
              onClick={() => onSelectContact(contact)}
              className={`w-full p-4 border-b border-gray-800 hover:bg-gray-800 transition text-left ${
                selectedContactId === contact.id ? 'bg-gray-800' : ''
              }`}
            >
              <div className="flex items-center gap-3">
                <div className="relative">
                  <div className="w-12 h-12 rounded-full bg-gradient-to-br from-purple-500 to-blue-500" />
                  <div className={`absolute bottom-0 right-0 w-3 h-3 rounded-full border-2 border-gray-900 ${getStatusColor(contact.status)}`} />
                </div>
                <div className="flex-1">
                  <p className="text-white font-medium">{contact.name}</p>
                  <p className="text-xs text-gray-500 font-mono">{contact.hash.substring(0, 16)}...</p>
                </div>
                {contact.verified && (
                  <span className="text-green-500 text-lg">✓</span>
                )}
              </div>
            </button>
          ))
        )}
      </div>

      {/* Add Contact Modal */}
      {showAddModal && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-gray-900 border border-gray-800 rounded-lg p-6 w-96">
            <h3 className="text-lg font-bold text-white mb-4">Add Contact</h3>
            <textarea
              value={newContactHash}
              onChange={(e) => setNewContactHash(e.target.value)}
              placeholder="Paste contact's public key..."
              className="w-full bg-gray-800 border border-gray-700 rounded px-3 py-2 text-white text-sm font-mono h-20 focus:outline-none focus:border-purple-500 mb-4"
            />
            <div className="flex gap-2">
              <button
                onClick={() => setShowAddModal(false)}
                className="flex-1 bg-gray-800 hover:bg-gray-700 text-white py-2 rounded transition"
              >
                Cancel
              </button>
              <button
                onClick={handleAddContact}
                className="flex-1 bg-purple-600 hover:bg-purple-700 text-white py-2 rounded transition"
              >
                Add
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
