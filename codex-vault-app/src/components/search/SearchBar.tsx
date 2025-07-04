import React, { useState, useRef, useEffect } from 'react';
import {
  MagnifyingGlassIcon,
  XMarkIcon,
  AdjustmentsHorizontalIcon,
  DocumentTextIcon,
} from '@heroicons/react/24/outline';
import { LoadingSpinner } from '../ui/LoadingSpinner';
import { useSearch } from '../../hooks/useApi';
import { SearchResult } from '../../services/api';
import clsx from 'clsx';

interface SearchBarProps {
  onSearch?: (query: string) => void;
  onResultSelect?: (result: SearchResult) => void;
  placeholder?: string;
  className?: string;
}

export function SearchBar({ onSearch, onResultSelect, placeholder = "Search documents...", className }: SearchBarProps) {
  const [query, setQuery] = useState('');
  const [showResults, setShowResults] = useState(false);
  const [showFilters, setShowFilters] = useState(false);
  const [searchType, setSearchType] = useState<'all' | 'title' | 'content'>('all');
  const [selectedCategory, setSelectedCategory] = useState<string>('');
  const inputRef = useRef<HTMLInputElement>(null);
  const resultsRef = useRef<HTMLDivElement>(null);

  // Use the search hook
  const { results, loading: isLoading, error, search, clearResults } = useSearch();

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        resultsRef.current &&
        !resultsRef.current.contains(event.target as Node) &&
        !inputRef.current?.contains(event.target as Node)
      ) {
        setShowResults(false);
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const handleInputChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    setQuery(value);

    if (value.trim().length > 2) {
      setShowResults(true);
      
      // Perform search with current filters
      search({
        query: value.trim(),
        category: selectedCategory || undefined,
        search_type: searchType,
        limit: 10
      });
    } else {
      clearResults();
      setShowResults(false);
    }
  };

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault();
    if (query.trim()) {
      onSearch?.(query.trim());
      setShowResults(false);
    }
  };

  const handleResultClick = (result: SearchResult) => {
    onResultSelect?.(result);
    setQuery(result.title);
    setShowResults(false);
  };

  const clearSearch = () => {
    setQuery('');
    clearResults();
    setShowResults(false);
    inputRef.current?.focus();
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Escape') {
      setShowResults(false);
    }
  };

  return (
    <div className={clsx('relative w-full max-w-2xl', className)}>
      <form onSubmit={handleSearch} className="relative">
        <div className="relative flex items-center">
          {/* Search Icon */}
          <MagnifyingGlassIcon className="absolute left-3 w-5 h-5 text-gray-400 dark:text-gray-500" />
          
          {/* Search Input */}
          <input
            ref={inputRef}
            type="text"
            value={query}
            onChange={handleInputChange}
            onKeyDown={handleKeyDown}
            onFocus={() => {
              if (results.length > 0) setShowResults(true);
            }}
            placeholder={placeholder}
            className={clsx(
              'w-full pl-10 pr-20 py-3 text-sm',
              'bg-white dark:bg-gray-800',
              'border border-gray-300 dark:border-gray-600',
              'rounded-lg shadow-sm',
              'placeholder-gray-500 dark:placeholder-gray-400',
              'text-gray-900 dark:text-white',
              'focus:ring-2 focus:ring-primary-500 focus:border-primary-500',
              'transition-colors duration-200'
            )}
          />
          
          {/* Loading Spinner */}
          {isLoading && (
            <div className="absolute right-12 flex items-center">
              <LoadingSpinner size="sm" />
            </div>
          )}
          
          {/* Clear Button */}
          {query && !isLoading && (
            <button
              type="button"
              onClick={clearSearch}
              className="absolute right-12 p-1 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 transition-colors"
            >
              <XMarkIcon className="w-4 h-4" />
            </button>
          )}
          
          {/* Filters Button */}
          <button
            type="button"
            onClick={() => setShowFilters(!showFilters)}
            className={clsx(
              'absolute right-3 p-1 rounded transition-colors',
              showFilters
                ? 'text-primary-600 dark:text-primary-400'
                : 'text-gray-400 hover:text-gray-600 dark:hover:text-gray-300'
            )}
          >
            <AdjustmentsHorizontalIcon className="w-4 h-4" />
          </button>
        </div>
      </form>

      {/* Search Results Dropdown */}
      {showResults && (
        <div
          ref={resultsRef}
          className={clsx(
            'absolute top-full left-0 right-0 mt-1 z-50',
            'bg-white dark:bg-gray-800',
            'border border-gray-200 dark:border-gray-700',
            'rounded-lg shadow-lg',
            'max-h-80 overflow-y-auto'
          )}
        >
          {isLoading ? (
            <div className="flex items-center justify-center p-4">
              <LoadingSpinner />
              <span className="ml-2 text-sm text-gray-500 dark:text-gray-400">
                Searching...
              </span>
            </div>
          ) : results.length > 0 ? (
            <div className="py-2">
              {results.map((result) => (
                <button
                  key={result.id}
                  onClick={() => handleResultClick(result)}
                  className="flex items-start w-full p-3 text-left hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
                >
                  <DocumentTextIcon className="w-5 h-5 text-gray-400 dark:text-gray-500 mt-0.5 mr-3 flex-shrink-0" />
                  <div className="flex-1 min-w-0">
                    <div className="text-sm font-medium text-gray-900 dark:text-white truncate">
                      {result.title}
                    </div>
                    {result.summary && (
                      <div className="text-xs text-gray-500 dark:text-gray-400 mt-1 line-clamp-2">
                        {result.summary}
                      </div>
                    )}
                    <div className="flex items-center mt-2 text-xs text-gray-500 dark:text-gray-400 space-x-3">
                      {result.category && (
                        <span>{result.category}</span>
                      )}
                      {result.reading_time && (
                        <span>{result.reading_time} min read</span>
                      )}
                    </div>
                  </div>
                </button>
              ))}
            </div>
          ) : query.trim().length > 2 ? (
            <div className="flex items-center justify-center p-4">
              <span className="text-sm text-gray-500 dark:text-gray-400">
                No documents found for "{query}"
              </span>
            </div>
          ) : null}
        </div>
      )}

      {/* Filters Panel */}
      {showFilters && (
        <div className={clsx(
          'absolute top-full left-0 right-0 mt-1 z-40',
          'bg-white dark:bg-gray-800',
          'border border-gray-200 dark:border-gray-700',
          'rounded-lg shadow-lg p-4'
        )}>
          <div className="text-sm font-medium text-gray-900 dark:text-white mb-3">
            Search Filters
          </div>
          
          {/* Category Filter */}
          <div className="mb-3">
            <label className="block text-xs text-gray-700 dark:text-gray-300 mb-1">
              Category
            </label>
            <select 
              value={selectedCategory}
              onChange={(e) => setSelectedCategory(e.target.value)}
              className="w-full p-2 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
            >
              <option value="">All Categories</option>
              <option value="philosophy">Philosophy</option>
              <option value="science">Science & Technology</option>
              <option value="history">History</option>
              <option value="literature">Literature</option>
            </select>
          </div>
          
          {/* Search Type */}
          <div className="mb-3">
            <label className="block text-xs text-gray-700 dark:text-gray-300 mb-1">
              Search Type
            </label>
            <div className="flex space-x-2">
              <button 
                onClick={() => setSearchType('all')}
                className={clsx(
                  'px-3 py-1 text-xs rounded',
                  searchType === 'all'
                    ? 'bg-primary-100 dark:bg-primary-900 text-primary-700 dark:text-primary-300'
                    : 'bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-gray-600'
                )}
              >
                All
              </button>
              <button 
                onClick={() => setSearchType('title')}
                className={clsx(
                  'px-3 py-1 text-xs rounded',
                  searchType === 'title'
                    ? 'bg-primary-100 dark:bg-primary-900 text-primary-700 dark:text-primary-300'
                    : 'bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-gray-600'
                )}
              >
                Title
              </button>
              <button 
                onClick={() => setSearchType('content')}
                className={clsx(
                  'px-3 py-1 text-xs rounded',
                  searchType === 'content'
                    ? 'bg-primary-100 dark:bg-primary-900 text-primary-700 dark:text-primary-300'
                    : 'bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-gray-600'
                )}
              >
                Content
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}