import {
  Component,
  OnInit,
  OnDestroy,
  ElementRef,
  ViewChild,
  AfterViewInit,
  ChangeDetectionStrategy,
  ChangeDetectorRef,
  isDevMode
} from '@angular/core';
import { Subject } from 'rxjs';
import { takeUntil } from 'rxjs/operators';
import { MapboxOverlay } from '@deck.gl/mapbox';
import { ScatterplotLayer, PolygonLayer } from '@deck.gl/layers';
import * as maplibregl from 'maplibre-gl';
import { FleetApiService } from '../../services/fleet-api.service';
import { FleetData, BikePosition, PollutionZone, TrafficJam } from '../../models/fleet.models';
import { WasmService } from '../../services/wasm.service';

@Component({
  selector: 'app-fleet-map',
  templateUrl: './fleet-map.component.html',
  styleUrls: ['./fleet-map.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush
})
export class FleetMapComponent implements OnInit, AfterViewInit, OnDestroy {
  @ViewChild('mapContainer', { static: true }) mapContainer!: ElementRef;

  private map!: maplibregl.Map;
  private deckOverlay!: MapboxOverlay;
  private destroy$ = new Subject<void>();

  // Cached static layers (pollution & traffic don't change)
  private cachedPollutionLayer: PolygonLayer<PollutionZone> | null = null;
  private cachedTrafficLayer: PolygonLayer<TrafficJam> | null = null;

  fleetData: FleetData | null = null;
  lastUpdate: Date | null = null;
  bikeCount = 0;
  deliveringCount = 0;
  updateCount = 0;
  selectedBikeId: string | null = null;

  // Amsterdam center - top-down 2D view
  private readonly INITIAL_VIEW = {
    longitude: 4.9041,
    latitude: 52.3676,
    zoom: 13,
    pitch: 0,
    bearing: 0
  };

  // Colors for visualization [R, G, B] or [R, G, B, A]
  private readonly BIKE_COLORS: { [key: string]: [number, number, number] } = {
    delivering: [46, 204, 113],   // Green
    returning: [52, 152, 219],    // Blue
    idle: [149, 165, 166]         // Gray
  };

  private readonly POLLUTION_COLORS: { [key: string]: [number, number, number, number] } = {
    high: [231, 76, 60, 120],       // Red
    moderate: [241, 196, 15, 100],  // Yellow
    low: [46, 204, 113, 80]         // Green
  };

  private readonly TRAFFIC_COLORS: { [key: string]: [number, number, number, number] } = {
    heavy: [192, 57, 43, 140],      // Dark red
    moderate: [230, 126, 34, 120],  // Orange
    light: [241, 196, 15, 100]      // Yellow
  };

  // WASM service for accelerated calculations
  private wasmInitialized = false;

  constructor(
    private fleetApiService: FleetApiService,
    private cdr: ChangeDetectorRef,
    private wasmService: WasmService
  ) {
    this.initializeWasm();
  }

  /**
   * Initialize WASM for hash calculations and statistics
   */
  private async initializeWasm(): Promise<void> {
    try {
      await this.wasmService.initialize();
      this.wasmInitialized = true;
    } catch (error) {
      console.warn('[FleetMapComponent] WASM not available, using TypeScript fallback');
    }
  }

  ngOnInit(): void {}

  ngAfterViewInit(): void {
    this.initializeMap();
  }

  private initializeMap(): void {
    // Initialize MapLibre GL map as the primary controller
    this.map = new maplibregl.Map({
      container: this.mapContainer.nativeElement,
      style: 'https://basemaps.cartocdn.com/gl/dark-matter-gl-style/style.json',
      center: [this.INITIAL_VIEW.longitude, this.INITIAL_VIEW.latitude],
      zoom: this.INITIAL_VIEW.zoom,
      pitch: this.INITIAL_VIEW.pitch,
      bearing: this.INITIAL_VIEW.bearing
    });

    // Add navigation controls
    this.map.addControl(new maplibregl.NavigationControl(), 'top-left');

    this.map.on('load', () => {
      this.initializeDeckOverlay();
      this.startDataStream();
    });
  }

  private initializeDeckOverlay(): void {
    // Use MapboxOverlay for seamless integration with MapLibre
    this.deckOverlay = new MapboxOverlay({
      interleaved: true,
      layers: []
    });

    this.map.addControl(this.deckOverlay as any);
  }

  private startDataStream(): void {
    this.fleetApiService.getFleetDataStream(5000)
      .pipe(takeUntil(this.destroy$))
      .subscribe(data => {
        this.updateCount++;
        this.fleetData = data;
        this.lastUpdate = data.timestamp;

        // Use WASM for statistics calculation (includes count by status)
        if (this.wasmInitialized) {
          try {
            const stats = this.wasmService.calculateFleetStatistics(data.bikes);
            this.bikeCount = stats.totalBikes;
            this.deliveringCount = stats.deliveringCount;
          } catch {
            // Fallback to manual calculation
            this.bikeCount = data.bikes.length;
            this.deliveringCount = data.bikes.filter(b => b.status === 'delivering').length;
          }
        } else {
          // TypeScript fallback
          this.bikeCount = data.bikes.length;
          this.deliveringCount = data.bikes.filter(b => b.status === 'delivering').length;
        }

        this.updateLayers(data);
        this.cdr.markForCheck();
        if (isDevMode()) {
          console.log(`[Fleet Update #${this.updateCount}] ${this.bikeCount} bikes, ${this.deliveringCount} delivering`);
        }
      });
  }

  private updateLayers(data: FleetData): void {
    // Cache static layers (pollution & traffic don't change during session)
    if (!this.cachedPollutionLayer) {
      this.cachedPollutionLayer = this.createPollutionLayer(data.pollutionZones);
    }
    if (!this.cachedTrafficLayer) {
      this.cachedTrafficLayer = this.createTrafficLayer(data.trafficJams);
    }

    // Only rebuild the dynamic bike layer on each update
    const layers = [
      this.cachedPollutionLayer,
      this.cachedTrafficLayer,
      this.createBikeLayer(data.bikes)
    ];

    this.deckOverlay.setProps({ layers });
  }

  private createPollutionLayer(zones: PollutionZone[]): PolygonLayer<PollutionZone> {
    return new PolygonLayer<PollutionZone>({
      id: 'pollution-layer',
      data: zones,
      pickable: true,
      stroked: true,
      filled: true,
      wireframe: false,
      lineWidthMinPixels: 2,
      getPolygon: (d: PollutionZone) => d.polygon,
      getFillColor: ((d: PollutionZone) => this.getPollutionColor(d.level)) as any,
      getLineColor: ((d: PollutionZone) => {
        const fill = this.getPollutionColor(d.level);
        return [fill[0], fill[1], fill[2], 255];
      }) as any,
      getLineWidth: 2
    });
  }

  private createTrafficLayer(jams: TrafficJam[]): PolygonLayer<TrafficJam> {
    return new PolygonLayer<TrafficJam>({
      id: 'traffic-layer',
      data: jams,
      pickable: true,
      stroked: true,
      filled: true,
      wireframe: false,
      lineWidthMinPixels: 2,
      getPolygon: (d: TrafficJam) => d.polygon,
      getFillColor: ((d: TrafficJam) => this.getTrafficColor(d.severity)) as any,
      getLineColor: ((d: TrafficJam) => {
        const fill = this.getTrafficColor(d.severity);
        return [fill[0], fill[1], fill[2], 255];
      }) as any,
      getLineWidth: 2
    });
  }

  private createBikeLayer(bikes: BikePosition[]): ScatterplotLayer<BikePosition> {
    return new ScatterplotLayer<BikePosition>({
      id: 'bike-layer',
      data: bikes,
      pickable: true,
      opacity: 0.9,
      stroked: true,
      filled: true,
      radiusScale: 1,
      radiusMinPixels: 8,
      radiusMaxPixels: 20,
      lineWidthMinPixels: 2,
      getPosition: (d: BikePosition) => [d.longitude, d.latitude],
      getRadius: (d: BikePosition) => this.selectedBikeId === d.id ? 80 : 50,
      getFillColor: ((d: BikePosition) => {
        if (this.selectedBikeId === d.id) {
          return [255, 215, 0]; // Gold highlight for selected bike
        }
        return this.getBikeColor(d.status);
      }) as any,
      getLineColor: ((d: BikePosition) => {
        if (this.selectedBikeId === d.id) {
          return [255, 255, 255, 255]; // Bright white border for selected
        }
        return [255, 255, 255, 180];
      }) as any,
      // Smooth animation when positions change
      transitions: {
        getPosition: {
          duration: 2000,
          easing: (t: number) => t * (2 - t) // ease-out quad
        }
      },
      // Ensure data changes trigger updates (using hash for performance)
      updateTriggers: {
        getPosition: this.hashBikePositions(bikes),
        getFillColor: this.selectedBikeId,
        getRadius: this.selectedBikeId,
        getLineColor: this.selectedBikeId
      }
    });
  }

  private getBikeColor(status: string): [number, number, number] {
    return this.BIKE_COLORS[status] || this.BIKE_COLORS['idle'];
  }

  private getPollutionColor(level: string): [number, number, number, number] {
    return this.POLLUTION_COLORS[level] || this.POLLUTION_COLORS['low'];
  }

  private getTrafficColor(severity: string): [number, number, number, number] {
    return this.TRAFFIC_COLORS[severity] || this.TRAFFIC_COLORS['light'];
  }

  /**
   * Fast hash of bike positions for deck.gl updateTriggers.
   *
   * Uses WASM FNV-1a algorithm when available, otherwise falls back
   * to TypeScript bit manipulation.
   */
  private hashBikePositions(bikes: BikePosition[]): number {
    // Use WASM hash for better performance and consistency
    if (this.wasmInitialized) {
      try {
        return this.wasmService.hashBikePositions(bikes);
      } catch {
        // Fall through to TypeScript implementation
      }
    }

    // TypeScript fallback - O(n) bit manipulation
    let hash = 0;
    for (const bike of bikes) {
      hash = ((hash << 5) - hash + (bike.longitude * 1000000) | 0) | 0;
      hash = ((hash << 5) - hash + (bike.latitude * 1000000) | 0) | 0;
    }
    return hash;
  }

  /**
   * Select a bike from the list - pans map to bike and highlights it
   */
  selectBike(bike: BikePosition): void {
    // Toggle selection if clicking same bike
    if (this.selectedBikeId === bike.id) {
      this.selectedBikeId = null;
    } else {
      this.selectedBikeId = bike.id;

      // Pan map to the selected bike with smooth animation
      this.map.flyTo({
        center: [bike.longitude, bike.latitude],
        zoom: 15,
        duration: 1000,
        essential: true
      });
    }

    // Re-render layers to show highlight
    if (this.fleetData) {
      this.updateLayers(this.fleetData);
    }
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
    if (this.map) {
      this.map.remove();
    }
  }
}
