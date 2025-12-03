/**
 * Session Durable Object
 * Manages user sessions with persistence
 */

interface Session {
  userId: string;
  email: string;
  createdAt: number;
  expiresAt: number;
}

export class SessionDO implements DurableObject {
  private state: DurableObjectState;

  constructor(state: DurableObjectState) {
    this.state = state;
  }

  async fetch(request: Request): Promise<Response> {
    const url = new URL(request.url);

    switch (url.pathname) {
      case '/create':
        return this.createSession(request);
      case '/get':
        return this.getSession();
      case '/refresh':
        return this.refreshSession();
      case '/destroy':
        return this.destroySession();
      default:
        return new Response('Not found', { status: 404 });
    }
  }

  private async createSession(request: Request): Promise<Response> {
    const { userId, email } = await request.json() as { userId: string; email: string };
    
    const session: Session = {
      userId,
      email,
      createdAt: Date.now(),
      expiresAt: Date.now() + 7 * 24 * 60 * 60 * 1000, // 7 days
    };
    
    await this.state.storage.put('session', session);
    return Response.json(session);
  }

  private async getSession(): Promise<Response> {
    const session = await this.state.storage.get<Session>('session');
    
    if (!session) {
      return Response.json({ error: 'No session' }, { status: 401 });
    }
    
    if (session.expiresAt < Date.now()) {
      await this.state.storage.delete('session');
      return Response.json({ error: 'Session expired' }, { status: 401 });
    }
    
    return Response.json(session);
  }

  private async refreshSession(): Promise<Response> {
    const session = await this.state.storage.get<Session>('session');
    
    if (!session || session.expiresAt < Date.now()) {
      return Response.json({ error: 'Invalid session' }, { status: 401 });
    }
    
    session.expiresAt = Date.now() + 7 * 24 * 60 * 60 * 1000;
    await this.state.storage.put('session', session);
    return Response.json(session);
  }

  private async destroySession(): Promise<Response> {
    await this.state.storage.delete('session');
    return Response.json({ success: true });
  }
}
