import { useState, useEffect, useCallback } from 'react';
import ApiService, { 
  Document, 
  SearchResult, 
  SearchOptions, 
  AiResponse, 
  SystemMetrics,
  ChatMessage,
  handleApiError,
  ApiError
} from '../services/api';

// Generic hook for API calls
export function useApiCall<T>(
  apiCall: () => Promise<T>,
  dependencies: any[] = []
) {
  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const execute = useCallback(async () => {
    setLoading(true);
    setError(null);
    
    try {
      const result = await apiCall();
      setData(result);
    } catch (err) {
      const apiError = handleApiError(err);
      setError(apiError.message);
    } finally {
      setLoading(false);
    }
  }, dependencies);

  useEffect(() => {
    execute();
  }, [execute]);

  return { data, loading, error, refetch: execute };
}

// Document management hooks
export function useDocument(documentId: string | null) {
  const [document, setDocument] = useState<Document | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadDocument = useCallback(async (id: string) => {
    if (!id) return;
    
    setLoading(true);
    setError(null);
    
    try {
      const doc = await ApiService.getDocument(id);
      setDocument(doc);
      
      // Increment view count
      await ApiService.incrementViewCount(id);
    } catch (err) {
      const apiError = handleApiError(err);
      setError(apiError.message);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (documentId) {
      loadDocument(documentId);
    } else {
      setDocument(null);
    }
  }, [documentId, loadDocument]);

  const toggleFavorite = useCallback(async () => {
    if (!document) return;
    
    try {
      const newFavoriteState = !document.is_favorite;
      await ApiService.toggleFavorite(document.id, newFavoriteState);
      setDocument(prev => prev ? { ...prev, is_favorite: newFavoriteState } : null);
    } catch (err) {
      const apiError = handleApiError(err);
      setError(apiError.message);
    }
  }, [document]);

  const toggleBookmark = useCallback(async () => {
    if (!document) return;
    
    try {
      const newBookmarkState = !document.is_bookmarked;
      await ApiService.toggleBookmark(document.id, newBookmarkState);
      setDocument(prev => prev ? { ...prev, is_bookmarked: newBookmarkState } : null);
    } catch (err) {
      const apiError = handleApiError(err);
      setError(apiError.message);
    }
  }, [document]);

  const shareDocument = useCallback(async (shareType: 'url' | 'text' | 'export') => {
    if (!document) return '';
    
    try {
      const shareData = await ApiService.shareDocument(document.id, shareType);
      return shareData;
    } catch (err) {
      const apiError = handleApiError(err);
      setError(apiError.message);
      return '';
    }
  }, [document]);

  return {
    document,
    loading,
    error,
    refetch: () => documentId && loadDocument(documentId),
    toggleFavorite,
    toggleBookmark,
    shareDocument
  };
}

// Search hook
export function useSearch() {
  const [results, setResults] = useState<SearchResult[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const search = useCallback(async (options: SearchOptions) => {
    setLoading(true);
    setError(null);
    
    try {
      const searchResults = await ApiService.searchDocuments(options);
      setResults(searchResults);
    } catch (err) {
      const apiError = handleApiError(err);
      setError(apiError.message);
      setResults([]);
    } finally {
      setLoading(false);
    }
  }, []);

  const clearResults = useCallback(() => {
    setResults([]);
    setError(null);
  }, []);

  return {
    results,
    loading,
    error,
    search,
    clearResults
  };
}

// Recent documents hook
export function useRecentDocuments(limit: number = 10) {
  return useApiCall(
    () => ApiService.getRecentDocuments(limit),
    [limit]
  );
}

// Favorite documents hook
export function useFavoriteDocuments() {
  return useApiCall(
    () => ApiService.getFavoriteDocuments(),
    []
  );
}

// Categories hook
export function useCategories() {
  return useApiCall(
    () => ApiService.getCategories(),
    []
  );
}

// AI Chat hook
export function useAiChat() {
  const [messages, setMessages] = useState<ChatMessage[]>([])
  const [isGenerating, setIsGenerating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const sendMessage = useCallback(async (prompt: string) => {
    if (!prompt.trim() || isGenerating) return;

    const userMessage = {
      id: Date.now().toString(),
      role: 'user' as const,
      content: prompt.trim(),
      timestamp: new Date()
    };

    setMessages(prev => [...prev, userMessage]);
    setIsGenerating(true);
    setError(null);

    try {
      const response = await ApiService.generateAiResponse(prompt);
      
      const assistantMessage = {
        id: (Date.now() + 1).toString(),
        role: 'assistant' as const,
        content: response.content,
        timestamp: new Date()
      };

      setMessages(prev => [...prev, assistantMessage]);
    } catch (err) {
      const apiError = handleApiError(err);
      setError(apiError.message);
    } finally {
      setIsGenerating(false);
    }
  }, [isGenerating]);

  const sendMessageStream = useCallback(async (prompt: string) => {
    if (!prompt.trim() || isGenerating) return;

    const userMessage = {
      id: Date.now().toString(),
      role: 'user' as const,
      content: prompt.trim(),
      timestamp: new Date()
    };

    setMessages(prev => [...prev, userMessage]);
    setIsGenerating(true);
    setError(null);

    const assistantId = (Date.now() + 1).toString();
    
    // Add empty assistant message for streaming
    setMessages(prev => [...prev, {
      id: assistantId,
      role: 'assistant' as const,
      content: '',
      timestamp: new Date()
    }]);

    try {
      await ApiService.generateAiResponseStream(
        prompt,
        (chunk) => {
          // Update streaming message
          setMessages(prev => prev.map(msg => 
            msg.id === assistantId 
              ? { ...msg, content: chunk }
              : msg
          ));
        },
        (response) => {
          // Final update
          setMessages(prev => prev.map(msg => 
            msg.id === assistantId 
              ? { ...msg, content: response.content }
              : msg
          ));
        },
        (error) => {
          setError(error);
        },
        messages // Pass current conversation history for context
      );
    } catch (err) {
      const apiError = handleApiError(err);
      setError(apiError.message);
    } finally {
      setIsGenerating(false);
    }
  }, [isGenerating]);

  const clearChat = useCallback(() => {
    setMessages([]);
    setError(null);
  }, []);

  return {
    messages,
    isGenerating,
    error,
    sendMessage,
    sendMessageStream,
    clearChat
  };
}

// System metrics hook
export function useSystemMetrics() {
  const [metrics, setMetrics] = useState<SystemMetrics | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchMetrics = useCallback(async () => {
    setLoading(true);
    setError(null);
    
    try {
      const systemMetrics = await ApiService.getSystemMetrics();
      setMetrics(systemMetrics);
    } catch (err) {
      const apiError = handleApiError(err);
      setError(apiError.message);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchMetrics();
    
    // Update metrics every 5 seconds
    const interval = setInterval(fetchMetrics, 5000);
    return () => clearInterval(interval);
  }, [fetchMetrics]);

  return {
    metrics,
    loading,
    error,
    refetch: fetchMetrics
  };
}

// Health check hook
export function useHealthCheck() {
  const [isHealthy, setIsHealthy] = useState<boolean | null>(null);
  const [loading, setLoading] = useState(false);

  const checkHealth = useCallback(async () => {
    setLoading(true);
    
    try {
      const healthy = await ApiService.healthCheck();
      setIsHealthy(healthy);
    } catch (err) {
      setIsHealthy(false);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    checkHealth();
    
    // Check health every 30 seconds
    const interval = setInterval(checkHealth, 30000);
    return () => clearInterval(interval);
  }, [checkHealth]);

  return {
    isHealthy,
    loading,
    refetch: checkHealth
  };
}

// Model management hook
export function useModelManagement() {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadModel = useCallback(async (modelPath?: string) => {
    setLoading(true);
    setError(null);
    
    try {
      await ApiService.loadModel(modelPath);
    } catch (err) {
      const apiError = handleApiError(err);
      setError(apiError.message);
    } finally {
      setLoading(false);
    }
  }, []);

  const unloadModel = useCallback(async () => {
    setLoading(true);
    setError(null);
    
    try {
      await ApiService.unloadModel();
    } catch (err) {
      const apiError = handleApiError(err);
      setError(apiError.message);
    } finally {
      setLoading(false);
    }
  }, []);

  return {
    loading,
    error,
    loadModel,
    unloadModel
  };
}