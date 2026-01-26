import { NgModule } from '@angular/core';
import { BrowserModule } from '@angular/platform-browser';
import { HttpClientModule, HTTP_INTERCEPTORS } from '@angular/common/http';

import { AppRoutingModule } from './app-routing.module';
import { AppComponent } from './app.component';
import { FleetMapComponent } from './components/fleet-map/fleet-map.component';
import { LegendComponent } from './components/legend/legend.component';
import { StatsPanelComponent } from './components/stats-panel/stats-panel.component';
import { BikeListPanelComponent } from './components/bike-list-panel/bike-list-panel.component';
import { BikeListItemComponent } from './components/bike-list-item/bike-list-item.component';
import { NavTabsComponent } from './components/nav-tabs/nav-tabs.component';
import { MockApiInterceptor } from './interceptors/mock-api.interceptor';

@NgModule({
  declarations: [
    AppComponent,
    FleetMapComponent,
    LegendComponent,
    StatsPanelComponent,
    BikeListPanelComponent,
    BikeListItemComponent,
    NavTabsComponent
    // DeliveriesPageComponent and IssuesPageComponent are standalone & lazy-loaded
  ],
  imports: [
    BrowserModule,
    HttpClientModule,
    AppRoutingModule
  ],
  providers: [
    {
      provide: HTTP_INTERCEPTORS,
      useClass: MockApiInterceptor,
      multi: true
    }
  ],
  bootstrap: [AppComponent]
})
export class AppModule { }
