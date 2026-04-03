import React, { useState } from 'react';
import { Lock, Key } from 'lucide-react';
import * as Crypto from '../crypto';

interface LoginProps {
  onLogin: (identity: Crypto.Identity) => void;
}

export const Login: React.FC<LoginProps> = ({ onLogin }) => {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [mode, setMode] = useState<'create' | 'import'>('create');
  const [publicKey, setPublicKey] = useState('');

  const handleCreateIdentity = async () => {
    try {
      setLoading(true);
      setError(null);
      
      // Generate new identity with X25519 + Ed25519
      const identity = await Crypto.generateIdentity();
      onLogin(identity);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create identity');
    } finally {
      setLoading(false);
    }
  };

  const handleImportIdentity = async () => {
    try {
      setLoading(true);
      setError(null);
      
      if (!publicKey.trim()) {
        setError('Please enter a public key');
        return;
      }

      // Import identity from public key
      const identity = await Crypto.importIdentity(publicKey);
      onLogin(identity);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to import identity');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-blue-900 via-purple-900 to-black flex items-center justify-center">
      <div className="w-full max-w-md">
        <div className="bg-gray-900 border border-purple-500 rounded-lg shadow-2xl p-8">
          {/* Header */}
          <div className="flex items-center justify-center mb-8">
            <div className="flex items-center gap-2">
              <Lock className="w-8 h-8 text-purple-500" />
              <h1 className="text-3xl font-bold text-white">NEXUS</h1>
            </div>
          </div>

          <p className="text-gray-400 text-center mb-6 text-sm">
            Post-Quantum Secure Messaging
          </p>

          {/* Mode Tabs */}
          <div className="flex gap-2 mb-6">
            <button
              onClick={() => setMode('create')}
              className={`flex-1 py-2 px-4 rounded transition ${
                mode === 'create'
                  ? 'bg-purple-600 text-white'
                  : 'bg-gray-800 text-gray-400 hover:bg-gray-700'
              }`}
            >
              Create New
            </button>
            <button
              onClick={() => setMode('import')}
              className={`flex-1 py-2 px-4 rounded transition ${
                mode === 'import'
                  ? 'bg-purple-600 text-white'
                  : 'bg-gray-800 text-gray-400 hover:bg-gray-700'
              }`}
            >
              Import
            </button>
          </div>

          {/* Error Message */}
          {error && (
            <div className="mb-4 p-3 bg-red-900 border border-red-500 rounded text-red-200 text-sm">
              {error}
            </div>
          )}

          {/* Content */}
          {mode === 'create' ? (
            <div className="space-y-4">
              <p className="text-gray-400 text-sm">
                Generate a new cryptographic identity using post-quantum algorithms (Kyber1024 + X25519).
              </p>
              <button
                onClick={handleCreateIdentity}
                disabled={loading}
                className="w-full bg-gradient-to-r from-purple-600 to-blue-600 hover:from-purple-700 hover:to-blue-700 disabled:opacity-50 text-white font-bold py-3 px-4 rounded transition flex items-center justify-center gap-2"
              >
                <Key className="w-5 h-5" />
                {loading ? 'Generating...' : 'Create Identity'}
              </button>
            </div>
          ) : (
            <div className="space-y-4">
              <div>
                <label className="block text-gray-400 text-sm font-medium mb-2">
                  Public Key (Base64)
                </label>
                <textarea
                  value={publicKey}
                  onChange={(e) => setPublicKey(e.target.value)}
                  placeholder="Paste your public key here..."
                  className="w-full bg-gray-800 border border-gray-700 rounded px-3 py-2 text-white text-sm font-mono h-24 focus:outline-none focus:border-purple-500"
                />
              </div>
              <button
                onClick={handleImportIdentity}
                disabled={loading}
                className="w-full bg-gradient-to-r from-purple-600 to-blue-600 hover:from-purple-700 hover:to-blue-700 disabled:opacity-50 text-white font-bold py-3 px-4 rounded transition flex items-center justify-center gap-2"
              >
                <Key className="w-5 h-5" />
                {loading ? 'Importing...' : 'Import Identity'}
              </button>
            </div>
          )}

          {/* Footer */}
          <p className="text-gray-500 text-xs text-center mt-6">
            Your keys are stored securely in browser storage with end-to-end encryption.
          </p>
        </div>
      </div>
    </div>
  );
};
