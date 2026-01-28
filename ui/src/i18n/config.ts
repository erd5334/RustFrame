import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';

import en from './locales/en.json';
import tr from './locales/tr.json';
import de from './locales/de.json';
import es from './locales/es.json';
import ja from './locales/ja.json';
import fr from './locales/fr.json';
import ru from './locales/ru.json';
import it from './locales/it.json';

// Get saved language from localStorage or default to 'en'
const getSavedLanguage = () => {
  try {
    return localStorage.getItem('rustframe_language') || 'en';
  } catch {
    return 'en';
  }
};

// Save language to localStorage
export const saveLanguage = (lang: string) => {
  try {
    localStorage.setItem('rustframe_language', lang);
  } catch (error) {
    console.error('Failed to save language preference:', error);
  }
};

i18n
  .use(initReactI18next)
  .init({
    resources: {
      en: { translation: en },
      tr: { translation: tr },
      de: { translation: de },
      es: { translation: es },
      ja: { translation: ja },
      fr: { translation: fr },
      ru: { translation: ru },
      it: { translation: it }
    },
    lng: getSavedLanguage(), // Use saved language
    fallbackLng: 'en',
    interpolation: {
      escapeValue: false,
    },
  });

export default i18n;
