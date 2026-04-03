import React, { useState } from 'react';
import { Settings, Copy, Download, LogOut, Lock, Bell, Shield } from 'lucide-react';

interface SettingsProps {
  publicKey: string;
  onLogout: () => void;
}

export const SettingsPanel: React.FC<SettingsProps> = ({ publicKey, onLogout }) => {
  const [copied, setCopied] = useState(false);
  const [notificationsEnabled, setNotificationsEnabled] = useState(true);
  const [autoLock, setAutoLock] = useState(true);

  const handleCopyKey = () => {
    navigator.clipboard.writeText(publicKey);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const handleDownloadKey = () => {
    const element = document.createElement('a');
    element.setAttribute('href', 'data:text/plain;charset=utf-8,' + encodeURIComponent(publicKey));
    element.setAttribute('download', 'nexus-public-key.txt');
    element.style.display = 'none';
    document.body.appendChild(element);
    element.click();
    document.body.removeChild(element);
  };

  return (
    <div className="w-full max-w-2xl mx-auto p-6">
      <div className="space-y-6">
        {/* Header */}
        <div className="flex items-center gap-2 mb-8">
          <Settings className="w-6 h-6 text-purple-500" />
          <h2 className="text-2xl font-bold text-white">Settings</h2>
        </div>

        {/* Security Section */}
        <div className="bg-gray-800 rounded-lg p-6 border border-gray-700">
          <div className="flex items-center gap-2 mb-4">
            <Lock className="w-5 h-5 text-purple-500" />
            <h3 className="text-lg font-bold text-white">Security</h3>
          </div>

          {/* Public Key */}
          <div className="mb-4">
            <label className="block text-gray-400 text-sm font-medium mb-2">
              Your Public Key
            </label>
            <div className="flex gap-2">
              <input
                type="text"
                value={publicKey.substring(0, 40) + '...'}
                readOnly
                className="flex-1 bg-gray-900 border border-gray-700 rounded px-3 py-2 text-white text-sm font-mono focus:outline-none"
              />
              <button
                onClick={handleCopyKey}
                className="bg-purple-600 hover:bg-purple-700 text-white px-4 py-2 rounded transition flex items-center gap-2"
              >
                <Copy className="w-4 h-4" />
                {copied ? 'Copied!' : 'Copy'}
              </button>
              <button
                onClick={handleDownloadKey}
                className="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded transition flex items-center gap-2"
              >
                <Download className="w-4 h-4" />
              </button>
            </div>
            <p className="text-xs text-gray-500 mt-2">
              Share this key with others to receive encrypted messages.
            </p>
          </div>

          {/* Auto-Lock Toggle */}
          <div className="flex items-center justify-between p-3 bg-gray-900 rounded">
            <label className="text-gray-300 font-medium">Auto-lock on inactivity</label>
            <input
              type="checkbox"
              checked={autoLock}
              onChange={(e) => setAutoLock(e.target.checked)}
              className="w-5 h-5 rounded"
            />
          </div>
        </div>

        {/* Notifications Section */}
        <div className="bg-gray-800 rounded-lg p-6 border border-gray-700">
          <div className="flex items-center gap-2 mb-4">
            <Bell className="w-5 h-5 text-purple-500" />
            <h3 className="text-lg font-bold text-white">Notifications</h3>
          </div>

          <div className="space-y-3">
            <div className="flex items-center justify-between p-3 bg-gray-900 rounded">
              <label className="text-gray-300 font-medium">Desktop notifications</label>
              <input
                type="checkbox"
                checked={notificationsEnabled}
                onChange={(e) => setNotificationsEnabled(e.target.checked)}
                className="w-5 h-5 rounded"
              />
            </div>
            <div className="flex items-center justify-between p-3 bg-gray-900 rounded">
              <label className="text-gray-300 font-medium">Message sounds</label>
              <input
                type="checkbox"
                defaultChecked
                className="w-5 h-5 rounded"
              />
            </div>
          </div>
        </div>

        {/* Privacy Section */}
        <div className="bg-gray-800 rounded-lg p-6 border border-gray-700">
          <div className="flex items-center gap-2 mb-4">
            <Shield className="w-5 h-5 text-purple-500" />
            <h3 className="text-lg font-bold text-white">Privacy</h3>
          </div>

          <div className="space-y-3">
            <div className="flex items-center justify-between p-3 bg-gray-900 rounded">
              <label className="text-gray-300 font-medium">Show online status</label>
              <input
                type="checkbox"
                defaultChecked
                className="w-5 h-5 rounded"
              />
            </div>
            <div className="flex items-center justify-between p-3 bg-gray-900 rounded">
              <label className="text-gray-300 font-medium">Allow message search</label>
              <input
                type="checkbox"
                defaultChecked
                className="w-5 h-5 rounded"
              />
            </div>
          </div>
        </div>

        {/* Danger Zone */}
        <div className="bg-red-900 bg-opacity-20 rounded-lg p-6 border border-red-800">
          <h3 className="text-lg font-bold text-red-400 mb-4">Danger Zone</h3>
          <button
            onClick={onLogout}
            className="w-full bg-red-600 hover:bg-red-700 text-white font-bold py-3 rounded transition flex items-center justify-center gap-2"
          >
            <LogOut className="w-5 h-5" />
            Logout
          </button>
        </div>
      </div>
    </div>
  );
};
