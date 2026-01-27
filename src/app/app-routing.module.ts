import { NgModule } from '@angular/core';
import { RouterModule, Routes } from '@angular/router';
import { FleetMapComponent } from './components/fleet-map/fleet-map.component';
import { licenseGuard, featureGuard } from './guards/license.guard';

const routes: Routes = [
  { path: '', redirectTo: '/map', pathMatch: 'full' },
  // Map requires basic license
  {
    path: 'map',
    component: FleetMapComponent,
    canActivate: [licenseGuard]
  },
  // Deliveries requires 'premium' feature
  {
    path: 'deliveries',
    loadComponent: () => import('./components/deliveries-page/deliveries-page.component')
      .then(m => m.DeliveriesPageComponent),
    canActivate: [featureGuard('premium')]
  },
  // Issues requires 'premium' feature
  {
    path: 'issues',
    loadComponent: () => import('./components/issues-page/issues-page.component')
      .then(m => m.IssuesPageComponent),
    canActivate: [featureGuard('premium')]
  },
  // License page is always accessible (for activation)
  {
    path: 'license',
    loadComponent: () => import('./components/license-dialog/license-dialog.component')
      .then(m => m.LicenseDialogComponent)
  },
  // Force graph requires 'api' feature
  {
    path: 'graph',
    loadComponent: () => import('./components/deliverer-graph-page/deliverer-graph-page.component')
      .then(m => m.DelivererGraphPageComponent),
    canActivate: [featureGuard('api')]
  },
];

@NgModule({
  imports: [RouterModule.forRoot(routes)],
  exports: [RouterModule]
})
export class AppRoutingModule { }
