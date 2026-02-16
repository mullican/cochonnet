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
                  <linearGradient id="redGradient" x1="0%" y1="0%" x2="100%" y2="100%">
                    <stop offset="0%" stopColor="#fca5a5" />
                    <stop offset="25%" stopColor="#ef4444" />
                    <stop offset="50%" stopColor="#dc2626" />
                    <stop offset="75%" stopColor="#ef4444" />
                    <stop offset="100%" stopColor="#b91c1c" />
                  </linearGradient>
                  <linearGradient id="redShineGradient" x1="0%" y1="0%" x2="100%" y2="100%">
                    <stop offset="0%" stopColor="white" stopOpacity="0.5" />
                    <stop offset="100%" stopColor="white" stopOpacity="0" />
                  </linearGradient>
                </defs>
                {/* Main steel boule */}
                <circle cx="32" cy="28" r="26" fill="url(#steelGradient)" />
                {/* Grooves on boule */}
                <path d="M10 22 Q32 18 54 22" fill="none" stroke="#374151" strokeWidth="1.5" strokeLinecap="round" />
                <path d="M8 30 Q32 26 56 30" fill="none" stroke="#374151" strokeWidth="1.5" strokeLinecap="round" />
                <path d="M10 38 Q32 34 54 38" fill="none" stroke="#374151" strokeWidth="1.5" strokeLinecap="round" />
                {/* Highlight shine on boule */}
                <ellipse cx="22" cy="18" rx="8" ry="5" fill="url(#shineGradient)" />
                {/* Red cochonnet in front */}
                <circle cx="32" cy="53" r="10" fill="url(#redGradient)" />
                {/* Highlight shine on cochonnet */}
                <ellipse cx="28" cy="49" rx="4" ry="2.5" fill="url(#redShineGradient)" />
              </svg>
              <span className="text-xl font-bold text-gray-900">Cochonnet</span>
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
