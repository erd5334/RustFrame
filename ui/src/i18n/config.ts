import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';

import en from './locales/en.json';

type LocaleEntry = {
  code: string;
  data: Record<string, unknown>;
};

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

export const i18nReady = i18n
  .use(initReactI18next)
  .init({
    resources: {
      en: { translation: en }
    },
    lng: getSavedLanguage(), // Use saved language
    fallbackLng: 'en',
    interpolation: {
      escapeValue: false,
    },
  });

export const listLocalLocales = async () => {
  try {
    return await invoke<string[]>('list_locales');
  } catch (error) {
    console.warn('Failed to list local locales:', error);
    return [];
  }
};

export const loadLocalLocales = async () => {
  await i18nReady;
  let locales: LocaleEntry[] = [];
  try {
    locales = await invoke<LocaleEntry[]>('load_locales');
  } catch (error) {
    console.warn('Failed to load local locales:', error);
  }

  const codes: string[] = [];
  for (const entry of locales) {
    if (!entry?.code || entry.code === 'en') {
      continue;
    }
    i18n.addResourceBundle(entry.code, 'translation', entry.data, true, true);
    codes.push(entry.code);
  }

  const available = new Set(['en', ...codes]);
  if (!available.has(i18n.language)) {
    i18n.changeLanguage('en');
    saveLanguage('en');
  }

  return codes;
};

export const getLocalesPath = async () => {
  try {
    return await invoke<string>('get_locales_path');
  } catch (error) {
    console.warn('Failed to get locales path:', error);
    return '';
  }
};

export default i18n;
