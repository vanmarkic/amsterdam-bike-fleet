import { NgModule } from '@angular/core';
import { RouterModule, Routes } from '@angular/router';
import { FleetMapComponent } from './components/fleet-map/fleet-map.component';

const routes: Routes = [
  { path: '', redirectTo: '/map', pathMatch: 'full' },
  { path: 'map', component: FleetMapComponent },
  // Lazy load deliveries and issues pages for better initial bundle size
  {
    path: 'deliveries',
    loadComponent: () => import('./components/deliveries-page/deliveries-page.component')
      .then(m => m.DeliveriesPageComponent)
  },
  {
    path: 'issues',
    loadComponent: () => import('./components/issues-page/issues-page.component')
      .then(m => m.IssuesPageComponent)
  },
  {
    path: 'license',
    loadComponent: () => import('./components/license-dialog/license-dialog.component')
      .then(m => m.LicenseDialogComponent)
  },
  // Force graph visualization for deliverer relationships
  {
    path: 'graph',
    loadComponent: () => import('./components/deliverer-graph-page/deliverer-graph-page.component')
      .then(m => m.DelivererGraphPageComponent)
  },
];

@NgModule({
  imports: [RouterModule.forRoot(routes)],
  exports: [RouterModule]
})
export class AppRoutingModule { }
