import { createContext, useContext, useState, useEffect, ReactNode } from 'react';
import { api } from '../lib/api';
import type { User } from '../types';

interface AuthContextType {
  user: User | null;
  loading: boolean;
  login: (email: string) => Promise<void>;
  register: (email: string, fullName?: string) => Promise<void>;
  logout: () => Promise<void>;
  refreshUser: (noCache?: boolean) => Promise<void>;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null);
  const [loading, setLoading] = useState(true);

  const refreshUser = async (noCache = false) => {
    try {
      const userData = await api.getMe(noCache);
      setUser(userData);
    } catch {
      api.setSession(null);
      setUser(null);
    }
  };

  useEffect(() => {
    const sessionId = api.getSession();
    if (sessionId) {
      refreshUser().finally(() => setLoading(false));
    } else {
      setLoading(false);
    }
  }, []);

  const login = async (email: string) => {
    await api.login(email);
    await refreshUser();
  };

  const register = async (email: string, fullName?: string) => {
    await api.register(email, fullName);
    await refreshUser();
  };

  const logout = async () => {
    try {
      await api.logout();
    } catch {
      // Even if server call fails, clear local state
      api.setSession(null);
    }
    setUser(null);
  };

  return (
    <AuthContext.Provider value={{ user, loading, login, register, logout, refreshUser }}>
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth() {
  const context = useContext(AuthContext);
  if (!context) throw new Error('useAuth must be used within AuthProvider');
  return context;
}
