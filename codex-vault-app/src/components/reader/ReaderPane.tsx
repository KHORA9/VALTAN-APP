import { useState } from 'react';
import {
  BookmarkIcon,
  HeartIcon,
  ShareIcon,
  EllipsisVerticalIcon,
  ClockIcon,
  TagIcon,
  EyeIcon,
  PrinterIcon,
  ArrowDownTrayIcon,
} from '@heroicons/react/24/outline';
import { HeartIcon as HeartSolidIcon, BookmarkIcon as BookmarkSolidIcon } from '@heroicons/react/24/solid';
import clsx from 'clsx';
import { LoadingSpinner } from '../ui/LoadingSpinner';
import { useDocument } from '../../hooks/useApi';

interface ReaderPaneProps {
  documentId?: string;
  className?: string;
  onFavoriteToggle?: (documentId: string, isFavorite: boolean) => void;
  onBookmarkToggle?: (documentId: string, isBookmarked: boolean) => void;
  onShare?: (documentId: string) => void;
}

export function ReaderPane({ 
  documentId, 
  className, 
  onFavoriteToggle, 
  onBookmarkToggle, 
  onShare 
}: ReaderPaneProps) {
  const [fontSize, setFontSize] = useState<'sm' | 'md' | 'lg'>('md');
  const [showActions, setShowActions] = useState(false);

  // Use the document hook
  const { 
    document, 
    loading, 
    error, 
    toggleFavorite, 
    toggleBookmark, 
    shareDocument 
  } = useDocument(documentId || null);

  const handleFavoriteToggle = async () => {
    await toggleFavorite();
    if (document) {
      onFavoriteToggle?.(document.id, !document.is_favorite);
    }
  };

  const handleBookmarkToggle = async () => {
    await toggleBookmark();
    if (document) {
      onBookmarkToggle?.(document.id, !document.is_bookmarked);
    }
  };

  const handleShare = async () => {
    if (document) {
      await shareDocument('url');
      onShare?.(document.id);
    }
  };

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString('en-US', {
      year: 'numeric',
      month: 'long',
      day: 'numeric'
    });
  };

  if (loading) {
    return (
      <div className={clsx('flex items-center justify-center h-full bg-white dark:bg-gray-900', className)}>
        <LoadingSpinner size="lg" />
      </div>
    );
  }

  if (error) {
    return (
      <div className={clsx('flex items-center justify-center h-full bg-white dark:bg-gray-900', className)}>
        <div className="text-center">
          <p className="text-red-600 dark:text-red-400 mb-2">{error}</p>
          <button
            onClick={() => window.location.reload()}
            className="px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors"
          >
            Try Again
          </button>
        </div>
      </div>
    );
  }

  if (!document) {
    return (
      <div className={clsx('flex items-center justify-center h-full bg-white dark:bg-gray-900', className)}>
        <div className="text-center max-w-md">
          <BookmarkIcon className="w-16 h-16 text-gray-400 dark:text-gray-500 mx-auto mb-4" />
          <h3 className="text-lg font-medium text-gray-900 dark:text-white mb-2">
            No Document Selected
          </h3>
          <p className="text-gray-500 dark:text-gray-400">
            Select a document from the sidebar to start reading
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className={clsx('flex flex-col h-full bg-white dark:bg-gray-900', className)}>
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b border-gray-200 dark:border-gray-800">
        <div className="flex-1 min-w-0">
          <h1 className="text-xl font-bold text-gray-900 dark:text-white truncate">
            {document.title}
          </h1>
          <div className="flex items-center mt-1 text-sm text-gray-500 dark:text-gray-400 space-x-4">
            {document.author && (
              <span>by {document.author}</span>
            )}
            <span>{formatDate(document.created_at)}</span>
            {document.reading_time && (
              <div className="flex items-center">
                <ClockIcon className="w-4 h-4 mr-1" />
                <span>{document.reading_time} min read</span>
              </div>
            )}
            {document.view_count && (
              <div className="flex items-center">
                <EyeIcon className="w-4 h-4 mr-1" />
                <span>{document.view_count.toLocaleString()} views</span>
              </div>
            )}
          </div>
        </div>
        
        {/* Actions */}
        <div className="flex items-center space-x-2 ml-4">
          <button
            onClick={handleFavoriteToggle}
            className={clsx(
              'p-2 rounded-lg transition-colors',
              document.is_favorite
                ? 'text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20'
                : 'text-gray-400 dark:text-gray-500 hover:bg-gray-100 dark:hover:bg-gray-800'
            )}
            title={document.is_favorite ? 'Remove from favorites' : 'Add to favorites'}
          >
            {document.is_favorite ? (
              <HeartSolidIcon className="w-5 h-5" />
            ) : (
              <HeartIcon className="w-5 h-5" />
            )}
          </button>
          
          <button
            onClick={handleBookmarkToggle}
            className={clsx(
              'p-2 rounded-lg transition-colors',
              document.is_bookmarked
                ? 'text-primary-600 dark:text-primary-400 hover:bg-primary-50 dark:hover:bg-primary-900/20'
                : 'text-gray-400 dark:text-gray-500 hover:bg-gray-100 dark:hover:bg-gray-800'
            )}
            title={document.is_bookmarked ? 'Remove bookmark' : 'Add bookmark'}
          >
            {document.is_bookmarked ? (
              <BookmarkSolidIcon className="w-5 h-5" />
            ) : (
              <BookmarkIcon className="w-5 h-5" />
            )}
          </button>
          
          <button
            onClick={handleShare}
            className="p-2 text-gray-400 dark:text-gray-500 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg transition-colors"
            title="Share document"
          >
            <ShareIcon className="w-5 h-5" />
          </button>
          
          <div className="relative">
            <button
              onClick={() => setShowActions(!showActions)}
              className="p-2 text-gray-400 dark:text-gray-500 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg transition-colors"
              title="More actions"
            >
              <EllipsisVerticalIcon className="w-5 h-5" />
            </button>
            
            {showActions && (
              <div className="absolute right-0 mt-2 w-48 bg-white dark:bg-gray-800 rounded-lg shadow-lg border border-gray-200 dark:border-gray-700 z-10">
                <div className="py-2">
                  <button className="flex items-center w-full px-4 py-2 text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                    <PrinterIcon className="w-4 h-4 mr-3" />
                    Print
                  </button>
                  <button className="flex items-center w-full px-4 py-2 text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                    <ArrowDownTrayIcon className="w-4 h-4 mr-3" />
                    Export
                  </button>
                </div>
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        <div className="max-w-4xl mx-auto p-6">
          {/* Tags */}
          {document.tags && document.tags.length > 0 && (
            <div className="flex items-center mb-6">
              <TagIcon className="w-4 h-4 text-gray-400 dark:text-gray-500 mr-2" />
              <div className="flex flex-wrap gap-2">
                {document.tags.map((tag) => (
                  <span
                    key={tag}
                    className="px-2 py-1 text-xs bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-400 rounded-full"
                  >
                    {tag}
                  </span>
                ))}
              </div>
            </div>
          )}

          {/* Document Content */}
          <div
            className={clsx(
              'prose dark:prose-invert max-w-none',
              {
                'prose-sm': fontSize === 'sm',
                'prose-base': fontSize === 'md',
                'prose-lg': fontSize === 'lg',
              }
            )}
          >
            <div
              dangerouslySetInnerHTML={{
                __html: document.content.replace(/\n/g, '<br />').replace(/^# (.*$)/gm, '<h1>$1</h1>').replace(/^## (.*$)/gm, '<h2>$1</h2>').replace(/^### (.*$)/gm, '<h3>$1</h3>')
              }}
            />
          </div>
        </div>
      </div>
      
      {/* Reading Controls */}
      <div className="flex items-center justify-between p-4 border-t border-gray-200 dark:border-gray-800">
        <div className="flex items-center space-x-2">
          <span className="text-sm text-gray-500 dark:text-gray-400">Font Size:</span>
          <div className="flex items-center space-x-1">
            {(['sm', 'md', 'lg'] as const).map((size) => (
              <button
                key={size}
                onClick={() => setFontSize(size)}
                className={clsx(
                  'px-2 py-1 text-xs rounded',
                  fontSize === size
                    ? 'bg-primary-100 dark:bg-primary-900 text-primary-700 dark:text-primary-300'
                    : 'bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-gray-700'
                )}
              >
                {size.toUpperCase()}
              </button>
            ))}
          </div>
        </div>
        
        <div className="flex items-center space-x-2 text-sm text-gray-500 dark:text-gray-400">
          <span>Category:</span>
          <span className="px-2 py-1 bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-400 rounded">
            {document.category}
          </span>
        </div>
      </div>
    </div>
  );
}