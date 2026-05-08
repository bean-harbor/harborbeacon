import { Routes } from '@angular/router';

import { HARBOR_ASSISTANT_PAGES } from './core/page-registry';
import { DeskPageComponent } from './pages/desk-page.component';

export const routes: Routes = [
  {
    path: '',
    pathMatch: 'full',
    redirectTo: HARBOR_ASSISTANT_PAGES[0].path
  },
  ...HARBOR_ASSISTANT_PAGES.map((page) => ({
    path: page.path,
    component: DeskPageComponent,
    data: {
      pageId: page.id
    }
  })),
  {
    path: '**',
    redirectTo: HARBOR_ASSISTANT_PAGES[0].path
  }
];
