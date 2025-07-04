import React, { useState, useEffect } from 'react';
import {
  DocumentTextIcon,
  HeartIcon,
  ClockIcon,
  FolderIcon,
  ChevronDownIcon,
  ChevronRightIcon,
} from '@heroicons/react/24/outline';
import { HeartIcon as HeartSolidIcon } from '@heroicons/react/24/solid';
import clsx from 'clsx';
import { LoadingSpinner } from '../ui/LoadingSpinner';
import { useRecentDocuments, useFavoriteDocuments, useCategories } from '../../hooks/useApi';
import { Document } from '../../services/api';

interface SidebarProps {
  className?: string;
  onDocumentSelect?: (documentId: string) => void;
  selectedDocumentId?: string;
}

export function Sidebar({ className, onDocumentSelect, selectedDocumentId }: SidebarProps) {
  const [expandedSections, setExpandedSections] = useState({
    recent: true,
    favorites: true,
    categories: false,
  });

  // Use API hooks
  const { data: recentDocuments, loading: recentLoading } = useRecentDocuments(5);
  const { data: favoriteDocuments, loading: favoritesLoading } = useFavoriteDocuments();
  const { data: categories, loading: categoriesLoading } = useCategories();

  const loading = recentLoading || favoritesLoading || categoriesLoading;

  const toggleSection = (section: keyof typeof expandedSections) => {
    setExpandedSections(prev => ({
      ...prev,
      [section]: !prev[section],
    }));
  };

  const handleDocumentClick = (documentId: string) => {
    onDocumentSelect?.(documentId);
  };

  // Helper function to get category color
  const getCategoryColor = (categoryName: string) => {
    const colorMap: { [key: string]: string } = {
      'Philosophy': 'bg-purple-500',
      'Science & Technology': 'bg-blue-500',
      'History': 'bg-green-500',
      'Literature': 'bg-yellow-500',
      'Health & Wellness': 'bg-pink-500',
      'Skills & Practical Knowledge': 'bg-orange-500',
    };
    return colorMap[categoryName] || 'bg-gray-500';
  };

  if (loading) {
    return (
      <div className={clsx('w-64 bg-white dark:bg-gray-900 border-r border-gray-200 dark:border-gray-800 flex items-center justify-center', className)}>
        <LoadingSpinner />
      </div>
    );
  }

  return (
    <div className={clsx('w-64 bg-white dark:bg-gray-900 border-r border-gray-200 dark:border-gray-800 flex flex-col', className)}>
      {/* Header */}
      <div className="p-4 border-b border-gray-200 dark:border-gray-800">
        <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
          Codex Vault
        </h2>
      </div>

      {/* Navigation */}
      <div className="flex-1 overflow-y-auto">
        {/* Recent Documents */}
        <div className="p-2">
          <button
            onClick={() => toggleSection('recent')}
            className="flex items-center justify-between w-full p-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-800 rounded-md"
          >
            <div className="flex items-center">
              <ClockIcon className="w-4 h-4 mr-2" />
              Recent
            </div>
            {expandedSections.recent ? (
              <ChevronDownIcon className="w-4 h-4" />
            ) : (
              <ChevronRightIcon className="w-4 h-4" />
            )}
          </button>
          
          {expandedSections.recent && (
            <div className="ml-6 mt-1 space-y-1">
              {recentDocuments?.map((doc) => (
                <button
                  key={doc.id}
                  onClick={() => handleDocumentClick(doc.id)}
                  className={clsx(
                    'flex items-start w-full p-2 text-sm text-left rounded-md transition-colors',
                    'hover:bg-gray-50 dark:hover:bg-gray-800',
                    selectedDocumentId === doc.id
                      ? 'bg-primary-50 dark:bg-primary-900/50 text-primary-700 dark:text-primary-300'
                      : 'text-gray-600 dark:text-gray-400'
                  )}
                >
                  <DocumentTextIcon className="w-4 h-4 mr-2 mt-0.5 flex-shrink-0" />
                  <div className="flex-1 min-w-0">
                    <div className="truncate font-medium">{doc.title}</div>
                    {doc.reading_time && (
                      <div className="text-xs text-gray-500 dark:text-gray-500">
                        {doc.reading_time} min read
                      </div>
                    )}
                  </div>
                  {doc.is_favorite && (
                    <HeartSolidIcon className="w-3 h-3 text-red-500 flex-shrink-0 ml-1" />
                  )}
                </button>
              ))}
            </div>
          )}
        </div>

        {/* Favorites */}
        <div className="p-2">
          <button
            onClick={() => toggleSection('favorites')}
            className="flex items-center justify-between w-full p-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-800 rounded-md"
          >
            <div className="flex items-center">
              <HeartIcon className="w-4 h-4 mr-2" />
              Favorites
            </div>
            {expandedSections.favorites ? (
              <ChevronDownIcon className="w-4 h-4" />
            ) : (
              <ChevronRightIcon className="w-4 h-4" />
            )}
          </button>
          
          {expandedSections.favorites && (
            <div className="ml-6 mt-1 space-y-1">
              {favoriteDocuments?.map((doc) => (
                <button
                  key={doc.id}
                  onClick={() => handleDocumentClick(doc.id)}
                  className={clsx(
                    'flex items-start w-full p-2 text-sm text-left rounded-md transition-colors',
                    'hover:bg-gray-50 dark:hover:bg-gray-800',
                    selectedDocumentId === doc.id
                      ? 'bg-primary-50 dark:bg-primary-900/50 text-primary-700 dark:text-primary-300'
                      : 'text-gray-600 dark:text-gray-400'
                  )}
                >
                  <DocumentTextIcon className="w-4 h-4 mr-2 mt-0.5 flex-shrink-0" />
                  <div className="flex-1 min-w-0">
                    <div className="truncate font-medium">{doc.title}</div>
                    <div className="text-xs text-gray-500 dark:text-gray-500">
                      {doc.category}
                    </div>
                  </div>
                </button>
              ))}
            </div>
          )}
        </div>

        {/* Categories */}
        <div className="p-2">
          <button
            onClick={() => toggleSection('categories')}
            className="flex items-center justify-between w-full p-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-800 rounded-md"
          >
            <div className="flex items-center">
              <FolderIcon className="w-4 h-4 mr-2" />
              Categories
            </div>
            {expandedSections.categories ? (
              <ChevronDownIcon className="w-4 h-4" />
            ) : (
              <ChevronRightIcon className="w-4 h-4" />
            )}
          </button>
          
          {expandedSections.categories && (
            <div className="ml-6 mt-1 space-y-1">
              {categories?.map((category) => (
                <button
                  key={category.name}
                  className="flex items-center w-full p-2 text-sm text-left text-gray-600 dark:text-gray-400 hover:bg-gray-50 dark:hover:bg-gray-800 rounded-md transition-colors"
                >
                  <div className={clsx('w-2 h-2 rounded-full mr-3', getCategoryColor(category.name))} />
                  <span className="flex-1 truncate">{category.name}</span>
                  <span className="text-xs text-gray-500 dark:text-gray-500">
                    {category.count}
                  </span>
                </button>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}