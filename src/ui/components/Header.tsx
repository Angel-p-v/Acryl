import { Box, Text } from 'ink';
import React from 'react';

import { theme } from '../theme.js';

export type HeaderProps = {
  title: string;
  subtitle?: string;
  count?: number;
};

/** Cabecera fija con el nombre de la aplicación y la vista activa. */
export function Header({ title, subtitle, count }: HeaderProps): React.JSX.Element {
  return (
    <Box justifyContent="space-between" paddingX={1}>
      <Text>
        {theme.title('media-manager')}
        {subtitle === undefined ? '' : theme.muted(` · ${subtitle}`)}
      </Text>
      <Text>{theme.muted(`${title}${count === undefined ? '' : ` · ${count} títulos`}`)}</Text>
    </Box>
  );
}
