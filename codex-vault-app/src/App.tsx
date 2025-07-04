import { useState, useEffect } from 'react';
import {
  Bars3Icon,
  XMarkIcon,
  ChatBubbleLeftRightIcon,
  DocumentTextIcon,
  Cog6ToothIcon,
} from '@heroicons/react/24/outline';
import clsx from 'clsx';

// Import components
import { Sidebar } from './components/layout/Sidebar';
import { SearchBar } from './components/search/SearchBar';
import { ReaderPane } from './components/reader/ReaderPane';
import { ChatPane } from './components/chat/ChatPane';
import { ThemeToggle } from './components/ui/ThemeToggle';
import { LoadingSpinner } from './components/ui/LoadingSpinner';

// Import types
import { SearchResult } from './services/api';

// Layout modes
type LayoutMode = 'reader' | 'chat' | 'split';
type ViewMode = 'desktop' | 'mobile';

function App() {
  // State management
  const [selectedDocumentId, setSelectedDocumentId] = useState<string | undefined>(undefined);
  const [layoutMode, setLayoutMode] = useState<LayoutMode>('reader');
  const [viewMode, setViewMode] = useState<ViewMode>('desktop');
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const [isLoading, setIsLoading] = useState(true);

  // Initialize app
  useEffect(() => {
    // Simulate app initialization
    const timer = setTimeout(() => {
      setIsLoading(false);
    }, 1000);

    return () => clearTimeout(timer);
  }, []);

  // Handle responsive design
  useEffect(() => {
    const handleResize = () => {
      const isMobile = window.innerWidth < 1024; // lg breakpoint
      setViewMode(isMobile ? 'mobile' : 'desktop');

      if (isMobile) {
        setSidebarCollapsed(true);
      }
    };

    handleResize();
    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);

  // Event handlers
  const handleDocumentSelect = (documentId: string) => {
    setSelectedDocumentId(documentId);
    if (viewMode === 'mobile') {
      setMobileMenuOpen(false);
      setLayoutMode('reader');
    }
  };

  const handleSearchResultSelect = (result: SearchResult) => {
    setSelectedDocumentId(result.id);
    if (viewMode === 'mobile') {
      setLayoutMode('reader');
    }
  };

  const handleSearch = (query: string) => {
    console.log('Search query:', query);
  };

  const handleSendMessage = (message: string) => {
    console.log('Chat message:', message);
  };

  const toggleSidebar = () => {
    if (viewMode === 'mobile') {
      setMobileMenuOpen(!mobileMenuOpen);
    } else {
      setSidebarCollapsed(!sidebarCollapsed);
    }
  };

  const toggleLayoutMode = (mode: LayoutMode) => {
    setLayoutMode(mode);
    if (viewMode === 'mobile' && mobileMenuOpen) {
      setMobileMenuOpen(false);
    }
  };

  // Loading screen
  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-screen bg-white dark:bg-gray-900">
        <div className="text-center">
          <LoadingSpinner size="lg" />
          <p className="mt-4 text-lg font-medium text-gray-700 dark:text-gray-300">
            Loading Codex Vault...
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-screen bg-gray-50 dark:bg-gray-900 overflow-hidden">
      {/* Mobile Sidebar Overlay */}
      {viewMode === 'mobile' && mobileMenuOpen && (
        <div className="fixed inset-0 z-50 lg:hidden">
          <div
            className="fixed inset-0 bg-black bg-opacity-50"
            onClick={() => setMobileMenuOpen(false)}
          />
          <div className="fixed inset-y-0 left-0 w-80 max-w-xs">
            <Sidebar
              onDocumentSelect={handleDocumentSelect}
              selectedDocumentId={selectedDocumentId}
            />
          </div>
        </div>
      )}

      {/* Desktop Sidebar */}
      {viewMode === 'desktop' && !sidebarCollapsed && (
        <Sidebar
          onDocumentSelect={handleDocumentSelect}
          selectedDocumentId={selectedDocumentId}
          className="flex-shrink-0"
        />
      )}

      {/* Main Content Area */}
      <div className="flex flex-col flex-1 min-w-0">
        {/* Top Navigation Bar */}
        <header className="flex items-center justify-between p-4 bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
          <div className="flex items-center space-x-4">
            {/* Menu Toggle */}
            <button
              onClick={toggleSidebar}
              className="p-2 text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
            >
              {(viewMode === 'mobile' && mobileMenuOpen) ||
              (viewMode === 'desktop' && !sidebarCollapsed) ? (
                <XMarkIcon className="w-5 h-5" />
              ) : (
                <Bars3Icon className="w-5 h-5" />
              )}
            </button>

            {/* Search Bar */}
            <div className="flex-1 max-w-2xl">
              <SearchBar
                onSearch={handleSearch}
                onResultSelect={handleSearchResultSelect}
                placeholder="Search documents..."
              />
            </div>
          </div>

          <div className="flex items-center space-x-2">
            {/* Layout Mode Toggles (Desktop Only) */}
            {viewMode === 'desktop' && (
              <div className="flex items-center bg-gray-100 dark:bg-gray-700 rounded-lg p-1">
                <button
                  onClick={() => toggleLayoutMode('reader')}
                  className={clsx(
                    'p-2 rounded-md transition-colors',
                    {
                      'bg-white dark:bg-gray-600 text-primary-600 dark:text-primary-400 shadow-sm':
                        layoutMode === 'reader',
                      'text-gray-500 dark:text-gray-400 hover:bg-white dark:hover:bg-gray-600':
                        layoutMode !== 'reader',
                    }
                  )}
                  title="Reader Mode"
                >
                  <DocumentTextIcon className="w-4 h-4" />
                </button>
                <button
                  onClick={() => toggleLayoutMode('chat')}
                  className={clsx(
                    'p-2 rounded-md transition-colors',
                    {
                      'bg-white dark:bg-gray-600 text-primary-600 dark:text-primary-400 shadow-sm':
                        layoutMode === 'chat',
                      'text-gray-500 dark:text-gray-400 hover:bg-white dark:hover:bg-gray-600':
                        layoutMode !== 'chat',
                    }
                  )}
                  title="Chat Mode"
                >
                  <ChatBubbleLeftRightIcon className="w-4 h-4" />
                </button>
                <button
                  onClick={() => toggleLayoutMode('split')}
                  className={clsx(
                    'p-2 rounded-md transition-colors',
                    layoutMode === 'split'
                      ? 'bg-white dark:bg-gray-600 text-primary-600 dark:text-primary-400 shadow-sm'
                      : 'text-gray-500 dark:text-gray-400 hover:bg-white dark:hover:bg-gray-600'
                  )}
                  title="Split Mode"
                >
                  <div className="w-4 h-4 flex">
                    <div className="w-2 h-4 border border-current rounded-l" />
                    <div className="w-2 h-4 border border-current rounded-r border-l-0" />
                  </div>
                </button>
              </div>
            )}

            {/* Mobile Layout Toggles */}
            {viewMode === 'mobile' && (
              <div className="flex items-center space-x-1">
                <button
                  onClick={() => toggleLayoutMode('reader')}
                  className={clsx(
                    'p-2 rounded-lg transition-colors',
                    layoutMode === 'reader'
                      ? 'bg-primary-100 dark:bg-primary-900 text-primary-600 dark:text-primary-400'
                      : 'text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700'
                  )}
                >
                  <DocumentTextIcon className="w-5 h-5" />
                </button>
                <button
                  onClick={() => toggleLayoutMode('chat')}
                  className={clsx(
                    'p-2 rounded-lg transition-colors',
                    layoutMode === 'chat'
                      ? 'bg-primary-100 dark:bg-primary-900 text-primary-600 dark:text-primary-400'
                      : 'text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700'
                  )}
                >
                  <ChatBubbleLeftRightIcon className="w-5 h-5" />
                </button>
              </div>
            )}

            {/* Settings and Theme */}
            <div className="flex items-center space-x-2">
              <ThemeToggle />
              <button className="p-2 text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors">
                <Cog6ToothIcon className="w-5 h-5" />
              </button>
            </div>
          </div>
        </header>

        {/* Main Content */}
        <main className="flex-1 flex min-h-0">
          {/* Desktop Layout */}
          {viewMode === 'desktop' && (
            <>
              {/* Reader Mode */}
              {layoutMode === 'reader' && (
                <ReaderPane
                  documentId={selectedDocumentId}
                  className="flex-1"
                />
              )}

              {/* Chat Mode */}
              {layoutMode === 'chat' && (
                <ChatPane
                  onSendMessage={handleSendMessage}
                  className="flex-1"
                />
              )}

              {/* Split Mode */}
              {layoutMode === 'split' && (
                <>
                  <ReaderPane
                    documentId={selectedDocumentId}
                    className="flex-1 border-r border-gray-200 dark:border-gray-700"
                  />
                  <ChatPane
                    onSendMessage={handleSendMessage}
                    className="flex-1"
                  />
                </>
              )}
            </>
          )}

          {/* Mobile Layout */}
          {viewMode === 'mobile' && (
            <>
              {layoutMode === 'reader' && (
                <ReaderPane
                  documentId={selectedDocumentId}
                  className="flex-1"
                />
              )}
              {layoutMode === 'chat' && (
                <ChatPane
                  onSendMessage={handleSendMessage}
                  className="flex-1"
                />
              )}
            </>
          )}
        </main>
      </div>
    </div>
  );
}

export default App;