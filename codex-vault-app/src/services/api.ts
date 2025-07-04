import { invoke } from '@tauri-apps/api/core';

// Types for API responses
export interface Document {
  id: string;
  title: string;
  content: string;
  category?: string;
  author?: string;
  created_at: string;
  reading_time?: number;
  is_favorite: boolean;
  is_bookmarked: boolean;
  view_count?: number;
  tags?: string[];
}

export interface SearchResult {
  id: string;
  title: string;
  summary?: string;
  category?: string;
  reading_time?: number;
  relevance_score?: number;
}

export interface SearchOptions {
  query: string;
  category?: string;
  search_type?: 'all' | 'title' | 'content';
  limit?: number;
  offset?: number;
}

export interface AiResponse {
  content: string;
  model: string;
  processing_time_ms: number;
  tokens_used: number;
}

export interface SystemMetrics {
  memory_usage_mb: number;
  cpu_usage_percent: number;
  model_loaded: boolean;
  cache_size_mb: number;
}

// API Service Class
export class ApiService {
  // Document Management
  static async searchDocuments(options: SearchOptions): Promise<SearchResult[]> {
    try {
      const results = await invoke<SearchResult[]>('search_documents', {
        query: options.query,
        category: options.category,
        searchType: options.search_type || 'all',
        limit: options.limit || 20,
        offset: options.offset || 0
      });
      return results;
    } catch (error) {
      console.error('Search documents error:', error);
      throw new Error(`Search failed: ${error}`);
    }
  }

  static async getDocument(documentId: string): Promise<Document> {
    try {
      const document = await invoke<Document>('get_document', { 
        documentId 
      });
      return document;
    } catch (error) {
      console.error('Get document error:', error);
      throw new Error(`Failed to load document: ${error}`);
    }
  }

  static async getRecentDocuments(limit: number = 10): Promise<Document[]> {
    try {
      const documents = await invoke<Document[]>('get_recent_documents', { 
        limit 
      });
      return documents;
    } catch (error) {
      console.error('Get recent documents error:', error);
      throw new Error(`Failed to load recent documents: ${error}`);
    }
  }

  static async getFavoriteDocuments(): Promise<Document[]> {
    try {
      const documents = await invoke<Document[]>('get_favorite_documents');
      return documents;
    } catch (error) {
      console.error('Get favorite documents error:', error);
      throw new Error(`Failed to load favorite documents: ${error}`);
    }
  }

  static async toggleFavorite(documentId: string, isFavorite: boolean): Promise<void> {
    try {
      await invoke('toggle_favorite', { 
        documentId, 
        isFavorite 
      });
    } catch (error) {
      console.error('Toggle favorite error:', error);
      throw new Error(`Failed to toggle favorite: ${error}`);
    }
  }

  static async toggleBookmark(documentId: string, isBookmarked: boolean): Promise<void> {
    try {
      await invoke('toggle_bookmark', { 
        documentId, 
        isBookmarked 
      });
    } catch (error) {
      console.error('Toggle bookmark error:', error);
      throw new Error(`Failed to toggle bookmark: ${error}`);
    }
  }

  // AI Integration
  static async generateAiResponse(prompt: string): Promise<AiResponse> {
    try {
      const response = await invoke<AiResponse>('generate_ai_response', { 
        prompt 
      });
      return response;
    } catch (error) {
      console.error('AI generation error:', error);
      throw new Error(`AI generation failed: ${error}`);
    }
  }

  static async generateAiResponseStream(
    prompt: string,
    onChunk: (chunk: string) => void,
    onComplete: (response: AiResponse) => void,
    onError: (error: string) => void
  ): Promise<void> {
    try {
      // Note: This would require implementing streaming support in Tauri
      // For now, we'll simulate streaming by calling the regular API
      const response = await this.generateAiResponse(prompt);
      
      // Simulate streaming by breaking response into chunks
      const words = response.content.split(' ');
      for (let i = 0; i < words.length; i++) {
        const chunk = words.slice(0, i + 1).join(' ');
        onChunk(chunk);
        await new Promise(resolve => setTimeout(resolve, 50));
      }
      
      onComplete(response);
    } catch (error) {
      onError(error instanceof Error ? error.message : String(error));
    }
  }

  // System Monitoring
  static async getSystemMetrics(): Promise<SystemMetrics> {
    try {
      const metrics = await invoke<SystemMetrics>('get_system_metrics');
      return metrics;
    } catch (error) {
      console.error('Get system metrics error:', error);
      throw new Error(`Failed to get system metrics: ${error}`);
    }
  }

  static async healthCheck(): Promise<boolean> {
    try {
      const isHealthy = await invoke<boolean>('health_check');
      return isHealthy;
    } catch (error) {
      console.error('Health check error:', error);
      return false;
    }
  }

  // Categories
  static async getCategories(): Promise<Array<{name: string, count: number}>> {
    try {
      const categories = await invoke<Array<{name: string, count: number}>>('get_categories');
      return categories;
    } catch (error) {
      console.error('Get categories error:', error);
      throw new Error(`Failed to load categories: ${error}`);
    }
  }

  // Content Management
  static async incrementViewCount(documentId: string): Promise<void> {
    try {
      await invoke('increment_view_count', { documentId });
    } catch (error) {
      console.error('Increment view count error:', error);
      // Non-critical error, don't throw
    }
  }

  static async shareDocument(documentId: string, shareType: 'url' | 'text' | 'export'): Promise<string> {
    try {
      const shareData = await invoke<string>('share_document', { 
        documentId, 
        shareType 
      });
      return shareData;
    } catch (error) {
      console.error('Share document error:', error);
      throw new Error(`Failed to share document: ${error}`);
    }
  }

  // Model Management
  static async loadModel(modelPath?: string): Promise<void> {
    try {
      await invoke('load_ai_model', { modelPath });
    } catch (error) {
      console.error('Load model error:', error);
      throw new Error(`Failed to load AI model: ${error}`);
    }
  }

  static async unloadModel(): Promise<void> {
    try {
      await invoke('unload_ai_model');
    } catch (error) {
      console.error('Unload model error:', error);
      throw new Error(`Failed to unload AI model: ${error}`);
    }
  }
}

// Error handling utilities
export class ApiError extends Error {
  constructor(
    message: string,
    public code?: string,
    public details?: any
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

// Hook for error handling
export const handleApiError = (error: any): ApiError => {
  if (error instanceof ApiError) {
    return error;
  }
  
  if (typeof error === 'string') {
    return new ApiError(error);
  }
  
  if (error?.message) {
    return new ApiError(error.message, error.code, error.details);
  }
  
  return new ApiError('An unknown error occurred', 'UNKNOWN_ERROR', error);
};

// Export default instance
export default ApiService;