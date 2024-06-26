import React, { useCallback, useRef, useMemo } from 'react';
import Typography from '@mui/material/Typography';
import KeyboardNavigation, {
  KeyboardNavigationOption,
} from '$app/components/_shared/keyboard_navigation/KeyboardNavigation';
import { useTranslation } from 'react-i18next';
import { TitleOutlined } from '@mui/icons-material';
import { EditorMarkFormat } from '$app/application/document/document.types';

export interface ColorPickerProps {
  onChange?: (format: EditorMarkFormat.FontColor | EditorMarkFormat.BgColor, color: string) => void;
  onEscape?: () => void;
  disableFocus?: boolean;
}
export function ColorPicker({ onEscape, onChange, disableFocus }: ColorPickerProps) {
  const { t } = useTranslation();

  const ref = useRef<HTMLDivElement>(null);

  const handleColorChange = useCallback(
    (key: string) => {
      const [format, , color = ''] = key.split('-');
      const formatKey = format === 'font' ? EditorMarkFormat.FontColor : EditorMarkFormat.BgColor;

      onChange?.(formatKey, color);
    },
    [onChange]
  );

  const renderColorItem = useCallback(
    (name: string, color: string, backgroundColor?: string) => {
      return (
        <div
          key={name}
          onClick={() => {
            handleColorChange(backgroundColor ? backgroundColor : color);
          }}
          className={'flex w-full cursor-pointer items-center justify-center gap-2'}
        >
          <div
            style={{
              backgroundColor: backgroundColor ?? 'transparent',
              color: color === '' ? 'var(--text-title)' : color,
            }}
            className={'flex h-5 w-5 items-center justify-center rounded border border-line-divider'}
          >
            <TitleOutlined className={'h-4 w-4'} />
          </div>
          <div className={'flex-1 text-xs text-text-title'}>{name}</div>
        </div>
      );
    },
    [handleColorChange]
  );

  const colors: KeyboardNavigationOption[] = useMemo(() => {
    return [
      {
        key: 'font_color',
        content: (
          <Typography className={'px-3 pb-1 pt-3 text-text-caption'} variant='subtitle2'>
            {t('editor.textColor')}
          </Typography>
        ),
        children: [
          {
            key: 'font-default',
            content: renderColorItem(t('editor.fontColorDefault'), ''),
          },
          {
            key: `font-gray-rgb(120, 119, 116)`,
            content: renderColorItem(t('editor.fontColorGray'), 'rgb(120, 119, 116)'),
          },
          {
            key: 'font-brown-rgb(159, 107, 83)',
            content: renderColorItem(t('editor.fontColorBrown'), 'rgb(159, 107, 83)'),
          },
          {
            key: 'font-orange-rgb(217, 115, 13)',
            content: renderColorItem(t('editor.fontColorOrange'), 'rgb(217, 115, 13)'),
          },
          {
            key: 'font-yellow-rgb(203, 145, 47)',
            content: renderColorItem(t('editor.fontColorYellow'), 'rgb(203, 145, 47)'),
          },
          {
            key: 'font-green-rgb(68, 131, 97)',
            content: renderColorItem(t('editor.fontColorGreen'), 'rgb(68, 131, 97)'),
          },
          {
            key: 'font-blue-rgb(51, 126, 169)',
            content: renderColorItem(t('editor.fontColorBlue'), 'rgb(51, 126, 169)'),
          },
          {
            key: 'font-purple-rgb(144, 101, 176)',
            content: renderColorItem(t('editor.fontColorPurple'), 'rgb(144, 101, 176)'),
          },
          {
            key: 'font-pink-rgb(193, 76, 138)',
            content: renderColorItem(t('editor.fontColorPink'), 'rgb(193, 76, 138)'),
          },
          {
            key: 'font-red-rgb(212, 76, 71)',
            content: renderColorItem(t('editor.fontColorRed'), 'rgb(212, 76, 71)'),
          },
        ],
      },
      {
        key: 'bg_color',
        content: (
          <Typography className={'px-3 pb-1 pt-3 text-text-caption'} variant='subtitle2'>
            {t('editor.backgroundColor')}
          </Typography>
        ),
        children: [
          {
            key: 'bg-default',
            content: renderColorItem(t('editor.backgroundColorDefault'), '', ''),
          },
          {
            key: `bg-gray-rgba(161,161,159,0.61)`,
            content: renderColorItem(t('editor.backgroundColorGray'), '', 'rgba(161,161,159,0.61)'),
          },
          {
            key: `bg-brown-rgba(178,93,37,0.65)`,
            content: renderColorItem(t('editor.backgroundColorBrown'), '', 'rgba(178,93,37,0.65)'),
          },
          {
            key: `bg-orange-rgba(248,156,71,0.65)`,
            content: renderColorItem(t('editor.backgroundColorOrange'), '', 'rgba(248,156,71,0.65)'),
          },
          {
            key: `bg-yellow-rgba(229,197,137,0.6)`,
            content: renderColorItem(t('editor.backgroundColorYellow'), '', 'rgba(229,197,137,0.6)'),
          },
          {
            key: `bg-green-rgba(124,189,111,0.65)`,
            content: renderColorItem(t('editor.backgroundColorGreen'), '', 'rgba(124,189,111,0.65)'),
          },
          {
            key: `bg-blue-rgba(100,174,199,0.71)`,
            content: renderColorItem(t('editor.backgroundColorBlue'), '', 'rgba(100,174,199,0.71)'),
          },
          {
            key: `bg-purple-rgba(182,114,234,0.63)`,
            content: renderColorItem(t('editor.backgroundColorPurple'), '', 'rgba(182,114,234,0.63)'),
          },
          {
            key: `bg-pink-rgba(238,142,179,0.6)`,
            content: renderColorItem(t('editor.backgroundColorPink'), '', 'rgba(238,142,179,0.6)'),
          },
          {
            key: `bg-red-rgba(238,88,98,0.64)`,
            content: renderColorItem(t('editor.backgroundColorRed'), '', 'rgba(238,88,98,0.64)'),
          },
        ],
      },
    ];
  }, [renderColorItem, t]);

  return (
    <div ref={ref} className={'flex h-full max-h-[360px] w-full flex-col overflow-y-auto'}>
      <KeyboardNavigation
        disableFocus={disableFocus}
        onPressLeft={onEscape}
        scrollRef={ref}
        options={colors}
        onConfirm={handleColorChange}
        onEscape={onEscape}
      />
    </div>
  );
}
