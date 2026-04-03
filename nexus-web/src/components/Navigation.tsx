// React Web App Navigation Component
// nexus-web/src/components/Navigation.tsx

import React, { useState } from 'react';
import { Menu, X, User, Settings, Bell, LogOut, Home, MessageSquare, Users, BarChart3 } from 'lucide-react';

interface NavigationProps {
  currentUser: {
    id: string;
    name: string;
    avatar: string;
    status: 'online' | 'away' | 'offline';
  };
  onLogout: () => void;
  unreadCount: number;
}

const Navigation: React.FC<NavigationProps> = ({ currentUser, onLogout, unreadCount }) => {
  const [isOpen, setIsOpen] = useState(false);
  const [showUserMenu, setShowUserMenu] = useState(false);
  const [notifications, setNotifications] = useState(3);

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'online':
        return 'bg-green-500';
      case 'away':
        return 'bg-yellow-500';
      case 'offline':
        return 'bg-gray-500';
      default:
        return 'bg-gray-500';
    }
  };

  return (
    <nav className="bg-white border-b border-gray-200 sticky top-0 z-50">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="flex justify-between items-center h-16">
          {/* Logo */}
          <div className="flex items-center space-x-3">
            <div className="text-2xl font-bold text-blue-600">🔒 Nexus</div>
            <span className="text-xs font-semibold text-gray-500 uppercase">v0.2.0</span>
          </div>

          {/* Desktop Menu */}
          <div className="hidden md:flex items-center space-x-1">
            <NavButton icon={Home} label="Home" href="/" />
            <NavButton icon={MessageSquare} label="Messages" href="/messages" />
            <NavButton icon={Users} label="Contacts" href="/contacts" />
            <NavButton icon={BarChart3} label="Admin" href="/admin" />
          </div>

          {/* User Menu & Settings */}
          <div className="flex items-center space-x-4">
            {/* Notification Bell */}
            <div className="relative">
              <button className="p-2 hover:bg-gray-100 rounded-lg transition relative">
                <Bell size={20} className="text-gray-600" />
                {notifications > 0 && (
                  <span className="absolute top-1 right-1 bg-red-500 text-white text-xs rounded-full h-5 w-5 flex items-center justify-center">
                    {notifications}
                  </span>
                )}
              </button>
            </div>

            {/* User Profile Button */}
            <div className="relative">
              <button
                onClick={() => setShowUserMenu(!showUserMenu)}
                className="flex items-center space-x-2 p-2 hover:bg-gray-100 rounded-lg transition"
              >
                <div className="relative">
                  <img
                    src={currentUser.avatar}
                    alt={currentUser.name}
                    className="h-8 w-8 rounded-full"
                  />
                  <div
                    className={`absolute bottom-0 right-0 h-3 w-3 rounded-full border-2 border-white ${getStatusColor(
                      currentUser.status
                    )}`}
                  />
                </div>
                <span className="text-sm font-medium text-gray-700 hidden sm:inline">
                  {currentUser.name}
                </span>
              </button>

              {/* User Dropdown Menu */}
              {showUserMenu && (
                <div className="absolute right-0 mt-2 w-48 bg-white rounded-lg shadow-lg border border-gray-200 py-1">
                  <div className="px-4 py-2 border-b border-gray-200">
                    <p className="text-sm font-semibold text-gray-900">{currentUser.name}</p>
                    <p className="text-xs text-gray-500">{currentUser.id}</p>
                  </div>

                  <button className="w-full text-left px-4 py-2 text-sm text-gray-700 hover:bg-gray-100 flex items-center space-x-2">
                    <User size={16} />
                    <span>Profile</span>
                  </button>

                  <button className="w-full text-left px-4 py-2 text-sm text-gray-700 hover:bg-gray-100 flex items-center space-x-2">
                    <Settings size={16} />
                    <span>Settings</span>
                  </button>

                  <div className="border-t border-gray-200 py-1">
                    <button
                      onClick={onLogout}
                      className="w-full text-left px-4 py-2 text-sm text-red-600 hover:bg-red-50 flex items-center space-x-2"
                    >
                      <LogOut size={16} />
                      <span>Sign Out</span>
                    </button>
                  </div>
                </div>
              )}
            </div>

            {/* Mobile Menu Button */}
            <button
              onClick={() => setIsOpen(!isOpen)}
              className="md:hidden p-2 hover:bg-gray-100 rounded-lg transition"
            >
              {isOpen ? <X size={20} /> : <Menu size={20} />}
            </button>
          </div>
        </div>
      </div>

      {/* Mobile Menu */}
      {isOpen && (
        <div className="md:hidden border-t border-gray-200 bg-white">
          <div className="px-2 pt-2 pb-3 space-y-1">
            <MobileNavButton icon={Home} label="Home" href="/" />
            <MobileNavButton icon={MessageSquare} label="Messages" href="/messages" />
            <MobileNavButton icon={Users} label="Contacts" href="/contacts" />
            <MobileNavButton icon={BarChart3} label="Admin" href="/admin" />
          </div>
        </div>
      )}
    </nav>
  );
};

interface NavButtonProps {
  icon: React.ComponentType<{ size: number }>;
  label: string;
  href: string;
}

const NavButton: React.FC<NavButtonProps> = ({ icon: Icon, label, href }) => (
  <a
    href={href}
    className="px-3 py-2 rounded-lg text-sm font-medium text-gray-700 hover:bg-gray-100 hover:text-gray-900 transition flex items-center space-x-1"
  >
    <Icon size={18} />
    <span>{label}</span>
  </a>
);

const MobileNavButton: React.FC<NavButtonProps> = ({ icon: Icon, label, href }) => (
  <a
    href={href}
    className="block px-3 py-2 rounded-md text-base font-medium text-gray-700 hover:bg-gray-100 hover:text-gray-900 transition flex items-center space-x-2"
  >
    <Icon size={18} />
    <span>{label}</span>
  </a>
);

export default Navigation;
