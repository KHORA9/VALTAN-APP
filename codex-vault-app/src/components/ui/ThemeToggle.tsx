import React from 'react';
import { SunIcon, MoonIcon, ComputerDesktopIcon } from '@heroicons/react/24/outline';
import { useTheme } from '../../contexts/ThemeContext';
import clsx from 'clsx';

export function ThemeToggle() {
  const { theme, setTheme } = useTheme();

  const themes = [
    { key: 'light' as const, icon: SunIcon, label: 'Light' },
    { key: 'dark' as const, icon: MoonIcon, label: 'Dark' },
    { key: 'system' as const, icon: ComputerDesktopIcon, label: 'System' },
  ];

  return (
    <div className="flex items-center bg-gray-100 dark:bg-gray-800 rounded-lg p-1">
      {themes.map(({ key, icon: Icon, label }) => (
        <button
          key={key}
          onClick={() => setTheme(key)}
          className={clsx(
            'flex items-center justify-center w-8 h-8 rounded-md transition-all duration-200',
            'hover:bg-white dark:hover:bg-gray-700',
            theme === key
              ? 'bg-white dark:bg-gray-700 text-primary-600 dark:text-primary-400 shadow-sm'
              : 'text-gray-500 dark:text-gray-400'
          )}
          title={`Switch to ${label.toLowerCase()} theme`}
        >
          <Icon className="w-4 h-4" />
        </button>
      ))}
    </div>
  );
}