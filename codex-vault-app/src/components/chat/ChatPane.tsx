import { useState, useRef, useEffect } from 'react';
import {
  PaperAirplaneIcon,
  StopIcon,
  TrashIcon,
  ClipboardDocumentIcon,
  SparklesIcon,
  ExclamationTriangleIcon,
  InformationCircleIcon,
} from '@heroicons/react/24/outline';
import { UserIcon, CpuChipIcon } from '@heroicons/react/24/solid';
import clsx from 'clsx';
import { LoadingSpinner } from '../ui/LoadingSpinner';
import { useAiChat, useHealthCheck } from '../../hooks/useApi';
import { ChatMessage } from '../../services/api';

interface ChatPaneProps {
  className?: string;
  onSendMessage?: (message: string) => void;
  onClearChat?: () => void;
  isGenerating?: boolean;
  systemStatus?: 'ready' | 'loading' | 'error';
}

export function ChatPane({
  className,
  onSendMessage,
  onClearChat,
}: ChatPaneProps) {
  const [inputMessage, setInputMessage] = useState('');
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  // Use hooks
  const { messages, isGenerating, sendMessage, clearChat } = useAiChat();
  const { isHealthy } = useHealthCheck();

  // Determine system status
  const systemStatus = isHealthy === true ? 'ready' : isHealthy === false ? 'error' : 'loading';

  useEffect(() => {
    scrollToBottom();
  }, [messages]);

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!inputMessage.trim() || isGenerating) return;

    const message = inputMessage.trim();
    setInputMessage('');
    
    // Call the parent callback
    onSendMessage?.(message);
    
    // Send message through API
    await sendMessage(message);
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit(e);
    }
  };

  const handleClear = () => {
    clearChat();
    onClearChat?.();
  };

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
  };

  const formatTime = (date: Date) => {
    return date.toLocaleTimeString('en-US', { 
      hour: '2-digit', 
      minute: '2-digit' 
    });
  };

  const renderMessage = (message: ChatMessage) => {
    const isUser = message.role === 'user';
    
    return (
      <div key={message.id} className={clsx('flex', isUser ? 'justify-end' : 'justify-start')}>
        <div className={clsx(
          'flex max-w-4xl space-x-3',
          isUser ? 'flex-row-reverse space-x-reverse' : 'flex-row'
        )}>
          {/* Avatar */}
          <div className={clsx(
            'flex-shrink-0 w-8 h-8 rounded-full flex items-center justify-center',
            isUser 
              ? 'bg-primary-600 text-white' 
              : 'bg-gray-200 dark:bg-gray-700'
          )}>
            {isUser ? (
              <UserIcon className="w-5 h-5" />
            ) : (
              <CpuChipIcon className="w-5 h-5 text-gray-600 dark:text-gray-300" />
            )}
          </div>

          {/* Message Content */}
          <div className={clsx(
            'flex flex-col space-y-2',
            isUser ? 'items-end' : 'items-start'
          )}>
            <div className={clsx(
              'rounded-lg px-4 py-2 max-w-prose',
              isUser
                ? 'bg-primary-600 text-white'
                : 'bg-gray-100 dark:bg-gray-800 text-gray-900 dark:text-white'
            )}>
              <div className="text-sm whitespace-pre-wrap">
                {message.content}
              </div>
            </div>
            
            {/* Message Actions */}
            <div className="flex items-center space-x-2 text-xs text-gray-500 dark:text-gray-400">
              <span>{formatTime(message.timestamp)}</span>
              {!isUser && (
                <button
                  onClick={() => copyToClipboard(message.content)}
                  className="hover:text-gray-700 dark:hover:text-gray-300 transition-colors"
                  title="Copy message"
                >
                  <ClipboardDocumentIcon className="w-4 h-4" />
                </button>
              )}
            </div>
          </div>
        </div>
      </div>
    );
  };

  return (
    <div className={clsx('flex flex-col h-full bg-white dark:bg-gray-900', className)}>
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b border-gray-200 dark:border-gray-800">
        <div className="flex items-center space-x-2">
          <SparklesIcon className="w-5 h-5 text-primary-600 dark:text-primary-400" />
          <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
            AI Assistant
          </h2>
          <div className={clsx(
            'flex items-center space-x-1 px-2 py-1 rounded-full text-xs',
            {
              'bg-green-100 dark:bg-green-900 text-green-700 dark:text-green-300': systemStatus === 'ready',
              'bg-yellow-100 dark:bg-yellow-900 text-yellow-700 dark:text-yellow-300': systemStatus === 'loading',
              'bg-red-100 dark:bg-red-900 text-red-700 dark:text-red-300': systemStatus === 'error'
            }
          )}>
            <div className={clsx(
              'w-2 h-2 rounded-full',
              {
                'bg-green-500': systemStatus === 'ready',
                'bg-yellow-500': systemStatus === 'loading',
                'bg-red-500': systemStatus === 'error'
              }
            )} />
            <span>{systemStatus === 'ready' ? 'Ready' : systemStatus === 'loading' ? 'Loading' : 'Error'}</span>
          </div>
        </div>
        
        <div className="flex items-center space-x-2">
          <button
            onClick={handleClear}
            className="p-2 text-gray-400 dark:text-gray-500 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg transition-colors"
            title="Clear conversation"
            disabled={messages.length === 0}
          >
            <TrashIcon className="w-5 h-5" />
          </button>
        </div>
      </div>

      {/* Messages */}
      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        {messages.length === 0 ? (
          <div className="flex items-center justify-center h-full">
            <div className="text-center max-w-md">
              <SparklesIcon className="w-16 h-16 text-gray-400 dark:text-gray-500 mx-auto mb-4" />
              <h3 className="text-lg font-medium text-gray-900 dark:text-white mb-2">
                Start a Conversation
              </h3>
              <p className="text-gray-500 dark:text-gray-400 mb-4">
                Ask me anything about the documents in your vault or explore new topics
              </p>
              <div className="flex flex-col space-y-2 text-sm text-gray-400 dark:text-gray-500">
                <div className="flex items-center">
                  <InformationCircleIcon className="w-4 h-4 mr-2" />
                  <span>All processing happens locally on your device</span>
                </div>
                <div className="flex items-center">
                  <ExclamationTriangleIcon className="w-4 h-4 mr-2" />
                  <span>No data is sent to external servers</span>
                </div>
              </div>
            </div>
          </div>
        ) : (
          <>
            {messages.map(renderMessage)}
            {isGenerating && (
              <div className="flex justify-start">
                <div className="flex max-w-4xl space-x-3">
                  <div className="flex-shrink-0 w-8 h-8 rounded-full bg-gray-200 dark:bg-gray-700 flex items-center justify-center">
                    <CpuChipIcon className="w-5 h-5 text-gray-600 dark:text-gray-300" />
                  </div>
                  <div className="flex items-center space-x-2 bg-gray-100 dark:bg-gray-800 rounded-lg px-4 py-2">
                    <LoadingSpinner size="sm" />
                    <span className="text-sm text-gray-500 dark:text-gray-400">
                      Thinking...
                    </span>
                  </div>
                </div>
              </div>
            )}
            <div ref={messagesEndRef} />
          </>
        )}
      </div>

      {/* Input Area */}
      <div className="p-4 border-t border-gray-200 dark:border-gray-800">
        <form onSubmit={handleSubmit} className="flex space-x-2">
          <div className="flex-1 relative">
            <textarea
              ref={inputRef}
              value={inputMessage}
              onChange={(e) => setInputMessage(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="Ask me anything..."
              className={clsx(
                'w-full px-4 py-3 pr-12 text-sm',
                'bg-gray-50 dark:bg-gray-800',
                'border border-gray-300 dark:border-gray-600',
                'rounded-lg resize-none',
                'placeholder-gray-500 dark:placeholder-gray-400',
                'text-gray-900 dark:text-white',
                'focus:ring-2 focus:ring-primary-500 focus:border-primary-500',
                'transition-colors duration-200'
              )}
              rows={1}
              style={{ minHeight: '48px', maxHeight: '120px' }}
              disabled={isGenerating}
            />
            
            {/* Character Count */}
            {inputMessage.length > 0 && (
              <div className="absolute bottom-2 right-2 text-xs text-gray-400 dark:text-gray-500">
                {inputMessage.length}
              </div>
            )}
          </div>
          
          <button
            type="submit"
            disabled={!inputMessage.trim() || isGenerating}
            className={clsx(
              'flex items-center justify-center w-12 h-12 rounded-lg transition-all',
              'focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary-500',
              inputMessage.trim() && !isGenerating
                ? 'bg-primary-600 hover:bg-primary-700 text-white'
                : 'bg-gray-200 dark:bg-gray-700 text-gray-400 dark:text-gray-500 cursor-not-allowed'
            )}
          >
            {isGenerating ? (
              <StopIcon className="w-5 h-5" />
            ) : (
              <PaperAirplaneIcon className="w-5 h-5" />
            )}
          </button>
        </form>
        
        {/* Input Hints */}
        <div className="mt-2 text-xs text-gray-500 dark:text-gray-400">
          <span>Press Enter to send, Shift+Enter for new line</span>
          {systemStatus === 'ready' && (
            <span className="ml-4">• Model loaded and ready</span>
          )}
        </div>
      </div>
    </div>
  );
}