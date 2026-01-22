import { Link, Outlet } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Select, SelectItem } from './ui';

export function Layout() {
  const { i18n } = useTranslation();

  const changeLanguage = (lang: string) => {
    i18n.changeLanguage(lang);
    localStorage.setItem('language', lang);
  };

  return (
    <div className="flex min-h-screen flex-col bg-gray-50">
      <header className="sticky top-0 z-40 border-b border-gray-200 bg-white">
        <div className="mx-auto flex h-16 max-w-7xl items-center justify-between px-4 sm:px-6 lg:px-8">
          <div className="flex items-center gap-8">
            <Link to="/" className="flex items-center gap-2">
              <svg
                xmlns="http://www.w3.org/2000/svg"
                width="32"
                height="32"
                viewBox="0 0 64 64"
              >
                {/* Steel boule with metallic gradient */}
                <defs>
                  <linearGradient id="steelGradient" x1="0%" y1="0%" x2="100%" y2="100%">
                    <stop offset="0%" stopColor="#e5e7eb" />
                    <stop offset="25%" stopColor="#9ca3af" />
                    <stop offset="50%" stopColor="#6b7280" />
                    <stop offset="75%" stopColor="#9ca3af" />
                    <stop offset="100%" stopColor="#4b5563" />
                  </linearGradient>
                  <linearGradient id="shineGradient" x1="0%" y1="0%" x2="100%" y2="100%">
                    <stop offset="0%" stopColor="white" stopOpacity="0.6" />
                    <stop offset="100%" stopColor="white" stopOpacity="0" />
                  </linearGradient>
                </defs>
                {/* Main sphere */}
                <circle cx="32" cy="32" r="28" fill="url(#steelGradient)" />
                {/* Grooves */}
                <path d="M8 26 Q32 22 56 26" fill="none" stroke="#374151" strokeWidth="2" strokeLinecap="round" />
                <path d="M6 34 Q32 30 58 34" fill="none" stroke="#374151" strokeWidth="2" strokeLinecap="round" />
                <path d="M8 42 Q32 38 56 42" fill="none" stroke="#374151" strokeWidth="2" strokeLinecap="round" />
                {/* Highlight shine */}
                <ellipse cx="22" cy="20" rx="10" ry="6" fill="url(#shineGradient)" />
              </svg>
              <span className="text-xl font-bold text-gray-900">Pétanque</span>
            </Link>
          </div>
          <div className="flex items-center gap-4">
            <Select
              value={i18n.language}
              onValueChange={changeLanguage}
            >
              <SelectItem value="en">English</SelectItem>
              <SelectItem value="fr">Français</SelectItem>
            </Select>
          </div>
        </div>
      </header>
      <main className="flex-1">
        <div className="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
          <Outlet />
        </div>
      </main>
    </div>
  );
}
