// React Admin Dashboard Component
// nexus-web/src/components/AdminDashboard.tsx

import React, { useState, useEffect } from 'react';
import { LineChart, Line, BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer } from 'recharts';

interface DashboardMetrics {
  totalUsers: number;
  activeUsers: number;
  messagesPerSecond: number;
  diskUsage: number;
  peakLatency: number;
  errorRate: number;
}

interface SystemHealth {
  cpuUsage: number;
  memoryUsage: number;
  diskUsage: number;
  networkLatency: number;
  databaseHealth: string;
}

const AdminDashboard: React.FC = () => {
  const [metrics, setMetrics] = useState<DashboardMetrics>({
    totalUsers: 1500000,
    activeUsers: 450000,
    messagesPerSecond: 8500,
    diskUsage: 2340,
    peakLatency: 45,
    errorRate: 0.02
  });

  const [systemHealth, setSystemHealth] = useState<SystemHealth>({
    cpuUsage: 65,
    memoryUsage: 71,
    diskUsage: 58,
    networkLatency: 12,
    databaseHealth: 'Healthy'
  });

  const [chartData] = useState([
    { time: '00:00', messages: 4200, users: 125000 },
    { time: '04:00', messages: 2800, users: 89000 },
    { time: '08:00', messages: 7200, users: 310000 },
    { time: '12:00', messages: 9500, users: 450000 },
    { time: '16:00', messages: 8900, users: 420000 },
    { time: '20:00', messages: 6700, users: 280000 },
    { time: '23:59', messages: 3400, users: 140000 }
  ]);

  useEffect(() => {
    // Fetch metrics from API
    const interval = setInterval(() => {
      // Simulate metric updates
      setMetrics(prev => ({
        ...prev,
        messagesPerSecond: Math.random() * 10000 + 5000,
        peakLatency: Math.random() * 100 + 20
      }));
    }, 5000);

    return () => clearInterval(interval);
  }, []);

  const getHealthColor = (value: number) => {
    if (value < 50) return '#10b981';
    if (value < 75) return '#f59e0b';
    return '#ef4444';
  };

  return (
    <div className="bg-gray-900 text-white p-8 min-h-screen">
      {/* Header */}
      <div className="mb-8">
        <h1 className="text-3xl font-bold mb-2">Admin Dashboard</h1>
        <p className="text-gray-400">System Overview & Metrics</p>
      </div>

      {/* Key Metrics */}
      <div className="grid grid-cols-1 md:grid-cols-3 lg:grid-cols-6 gap-4 mb-8">
        <MetricCard
          title="Total Users"
          value={`${(metrics.totalUsers / 1000000).toFixed(1)}M`}
          change="+2.5%"
          icon="👥"
        />
        <MetricCard
          title="Active Users"
          value={`${(metrics.activeUsers / 1000).toFixed(0)}K`}
          change="+5.2%"
          icon="🟢"
        />
        <MetricCard
          title="Messages/sec"
          value={Math.floor(metrics.messagesPerSecond).toLocaleString()}
          change="+1.8%"
          icon="💬"
        />
        <MetricCard
          title="Peak Latency"
          value={`${metrics.peakLatency.toFixed(0)}ms`}
          change="-3.1%"
          icon="⚡"
        />
        <MetricCard
          title="Error Rate"
          value={`${metrics.errorRate.toFixed(3)}%`}
          change="-0.5%"
          icon="❌"
        />
        <MetricCard
          title="Disk Usage"
          value={`${metrics.diskUsage}GB`}
          change="+0.8%"
          icon="💾"
        />
      </div>

      {/* Charts */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-8 mb-8">
        {/* Messages Timeline */}
        <div className="bg-gray-800 rounded-lg p-6">
          <h2 className="text-xl font-semibold mb-4">Message Traffic (24h)</h2>
          <ResponsiveContainer width="100%" height={300}>
            <LineChart data={chartData}>
              <CartesianGrid strokeDasharray="3 3" stroke="#444" />
              <XAxis dataKey="time" stroke="#888" />
              <YAxis stroke="#888" />
              <Tooltip contentStyle={{ backgroundColor: '#1f2937', border: 'none' }} />
              <Legend />
              <Line
                type="monotone"
                dataKey="messages"
                stroke="#3b82f6"
                strokeWidth={2}
                dot={{ fill: '#3b82f6', r: 4 }}
              />
            </LineChart>
          </ResponsiveContainer>
        </div>

        {/* Active Users */}
        <div className="bg-gray-800 rounded-lg p-6">
          <h2 className="text-xl font-semibold mb-4">Active Users (24h)</h2>
          <ResponsiveContainer width="100%" height={300}>
            <BarChart data={chartData}>
              <CartesianGrid strokeDasharray="3 3" stroke="#444" />
              <XAxis dataKey="time" stroke="#888" />
              <YAxis stroke="#888" />
              <Tooltip contentStyle={{ backgroundColor: '#1f2937', border: 'none' }} />
              <Bar dataKey="users" fill="#10b981" />
            </BarChart>
          </ResponsiveContainer>
        </div>
      </div>

      {/* System Health */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
        {/* Resource Usage */}
        <div className="bg-gray-800 rounded-lg p-6">
          <h2 className="text-xl font-semibold mb-4">Resource Usage</h2>
          <div className="space-y-4">
            <ResourceBar label="CPU Usage" value={systemHealth.cpuUsage} />
            <ResourceBar label="Memory Usage" value={systemHealth.memoryUsage} />
            <ResourceBar label="Disk Usage" value={systemHealth.diskUsage} />
            <ResourceBar label="Network Latency" value={systemHealth.networkLatency} />
          </div>
        </div>

        {/* System Status */}
        <div className="bg-gray-800 rounded-lg p-6">
          <h2 className="text-xl font-semibold mb-4">System Status</h2>
          <div className="space-y-3">
            <StatusItem label="API Server" status="healthy" />
            <StatusItem label="Relay Server" status="healthy" />
            <StatusItem label="Database" status="healthy" />
            <StatusItem label="Cache Layer" status="healthy" />
            <StatusItem label="Message Queue" status="healthy" />
            <StatusItem label="Backup Service" status="healthy" />
          </div>
        </div>
      </div>

      {/* Recent Activity */}
      <div className="bg-gray-800 rounded-lg p-6 mt-8">
        <h2 className="text-xl font-semibold mb-4">Recent Events</h2>
        <div className="space-y-2">
          <EventLog timestamp="14:32:15" event="User authentication spike - 2.5M login attempts" severity="warning" />
          <EventLog timestamp="14:15:43" event="Database query optimization completed" severity="info" />
          <EventLog timestamp="13:58:22" event="New deployment: v0.2.0 released" severity="success" />
          <EventLog timestamp="13:42:10" event="Load balancer rebalanced - 15% CPU reduction" severity="success" />
          <EventLog timestamp="13:15:33" event="Cache hit rate improved to 96.8%" severity="success" />
        </div>
      </div>
    </div>
  );
};

interface MetricCardProps {
  title: string;
  value: string;
  change: string;
  icon: string;
}

const MetricCard: React.FC<MetricCardProps> = ({ title, value, change, icon }) => (
  <div className="bg-gray-800 rounded-lg p-4">
    <div className="flex items-center justify-between mb-2">
      <span className="text-gray-400 text-sm">{title}</span>
      <span className="text-2xl">{icon}</span>
    </div>
    <div className="text-2xl font-bold">{value}</div>
    <div className="text-green-400 text-sm mt-1">{change} from last hour</div>
  </div>
);

interface ResourceBarProps {
  label: string;
  value: number;
}

const ResourceBar: React.FC<ResourceBarProps> = ({ label, value }) => (
  <div>
    <div className="flex justify-between mb-1">
      <span className="text-sm">{label}</span>
      <span className="text-sm font-semibold">{value}%</span>
    </div>
    <div className="bg-gray-700 rounded-full h-2 overflow-hidden">
      <div
        className="h-full transition-all duration-300"
        style={{
          width: `${value}%`,
          backgroundColor: value < 50 ? '#10b981' : value < 75 ? '#f59e0b' : '#ef4444'
        }}
      />
    </div>
  </div>
);

interface StatusItemProps {
  label: string;
  status: 'healthy' | 'warning' | 'critical';
}

const StatusItem: React.FC<StatusItemProps> = ({ label, status }) => {
  const statusColors = {
    healthy: 'bg-green-900 text-green-200',
    warning: 'bg-yellow-900 text-yellow-200',
    critical: 'bg-red-900 text-red-200'
  };

  const statusDots = {
    healthy: '🟢',
    warning: '🟡',
    critical: '🔴'
  };

  return (
    <div className="flex items-center justify-between p-3 bg-gray-700 rounded">
      <span>{label}</span>
      <span className={`px-3 py-1 rounded-full text-sm ${statusColors[status]}`}>
        {statusDots[status]} {status.charAt(0).toUpperCase() + status.slice(1)}
      </span>
    </div>
  );
};

interface EventLogProps {
  timestamp: string;
  event: string;
  severity: 'info' | 'warning' | 'error' | 'success';
}

const EventLog: React.FC<EventLogProps> = ({ timestamp, event, severity }) => {
  const severityColors = {
    info: 'text-blue-400',
    warning: 'text-yellow-400',
    error: 'text-red-400',
    success: 'text-green-400'
  };

  return (
    <div className="flex gap-4 text-sm">
      <span className="text-gray-500 min-w-fit">{timestamp}</span>
      <span className={severityColors[severity]}>{event}</span>
    </div>
  );
};

export default AdminDashboard;
