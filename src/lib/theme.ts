let current: 'dark' | 'light' = 'dark';

export function getTheme(): 'dark' | 'light' {
  return current;
}

export function setTheme(theme: 'dark' | 'light') {
  current = theme;
  if (theme === 'light') {
    document.documentElement.classList.add('light');
  } else {
    document.documentElement.classList.remove('light');
  }
  localStorage.setItem('git-explorer-theme', theme);
}

export function initTheme() {
  const saved = localStorage.getItem('git-explorer-theme') as 'dark' | 'light' | null;
  setTheme(saved ?? 'dark');
}

export function toggleTheme() {
  setTheme(current === 'dark' ? 'light' : 'dark');
}
