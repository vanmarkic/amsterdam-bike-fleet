import { Injectable } from '@angular/core';
import { Observable, of, interval } from 'rxjs';
import { map, startWith } from 'rxjs/operators';
import { BikePosition, PollutionZone, TrafficJam, FleetData } from '../models/fleet.models';

@Injectable({
  providedIn: 'root'
})
export class FleetApiService {
  // Amsterdam center coordinates
  private readonly AMSTERDAM_CENTER = { lng: 4.9041, lat: 52.3676 };

  // Bike courier names (Dutch-style)
  private readonly COURIER_NAMES = [
    'Jan', 'Pieter', 'Klaas', 'Willem', 'Daan', 'Bram', 'Lars', 'Thijs',
    'Sophie', 'Emma', 'Julia', 'Anna', 'Lisa', 'Femke', 'Lotte', 'Eva',
    'Max', 'Finn', 'Luuk', 'Sander'
  ];

  // Fixed pollution zones (based on real Amsterdam hotspots)
  private readonly POLLUTION_ZONES: PollutionZone[] = [
    {
      id: 'pollution-1',
      name: 'A10 Ring West',
      level: 'high',
      polygon: [
        [4.8350, 52.3750], [4.8450, 52.3750], [4.8450, 52.3650], [4.8350, 52.3650]
      ]
    },
    {
      id: 'pollution-2',
      name: 'Centraal Station Area',
      level: 'moderate',
      polygon: [
        [4.8950, 52.3800], [4.9150, 52.3800], [4.9150, 52.3750], [4.8950, 52.3750]
      ]
    },
    {
      id: 'pollution-3',
      name: 'Amstel Industrial',
      level: 'high',
      polygon: [
        [4.9200, 52.3400], [4.9400, 52.3400], [4.9400, 52.3300], [4.9200, 52.3300]
      ]
    }
  ];

  // Fixed traffic jam zones (based on typical Amsterdam congestion points)
  private readonly TRAFFIC_JAMS: TrafficJam[] = [
    {
      id: 'traffic-1',
      name: 'Prins Hendrikkade',
      severity: 'heavy',
      polygon: [
        [4.8980, 52.3770], [4.9100, 52.3770], [4.9100, 52.3755], [4.8980, 52.3755]
      ]
    },
    {
      id: 'traffic-2',
      name: 'Overtoom Crossing',
      severity: 'moderate',
      polygon: [
        [4.8700, 52.3620], [4.8780, 52.3620], [4.8780, 52.3600], [4.8700, 52.3600]
      ]
    },
    {
      id: 'traffic-3',
      name: 'Stadhouderskade',
      severity: 'light',
      polygon: [
        [4.8800, 52.3580], [4.8950, 52.3580], [4.8950, 52.3565], [4.8800, 52.3565]
      ]
    },
    {
      id: 'traffic-4',
      name: 'Wibautstraat',
      severity: 'heavy',
      polygon: [
        [4.9150, 52.3550], [4.9200, 52.3550], [4.9200, 52.3480], [4.9150, 52.3480]
      ]
    }
  ];

  private bikePositions: BikePosition[] = [];

  constructor() {
    this.initializeBikes();
  }

  private initializeBikes(): void {
    this.bikePositions = this.COURIER_NAMES.map((name, index) => ({
      id: `bike-${index + 1}`,
      name,
      longitude: this.AMSTERDAM_CENTER.lng + (Math.random() - 0.5) * 0.08,
      latitude: this.AMSTERDAM_CENTER.lat + (Math.random() - 0.5) * 0.06,
      status: this.getRandomStatus(),
      speed: Math.floor(Math.random() * 25) + 5
    }));
  }

  private getRandomStatus(): 'delivering' | 'idle' | 'returning' {
    const rand = Math.random();
    if (rand < 0.5) return 'delivering';
    if (rand < 0.8) return 'returning';
    return 'idle';
  }

  private updateBikePositions(): void {
    this.bikePositions = this.bikePositions.map(bike => {
      // Simulate movement (bikes move ~0.001 degrees per update = ~100m)
      // More visible movement for demo purposes
      const movement = bike.status === 'idle' ? 0.0002 : 0.001;
      const angle = Math.random() * Math.PI * 2;

      let newLng = bike.longitude + Math.cos(angle) * movement;
      let newLat = bike.latitude + Math.sin(angle) * movement;

      // Keep bikes within Amsterdam bounds
      newLng = Math.max(4.85, Math.min(4.95, newLng));
      newLat = Math.max(52.34, Math.min(52.40, newLat));

      // Occasionally change status
      let newStatus = bike.status;
      if (Math.random() < 0.1) {
        newStatus = this.getRandomStatus();
      }

      // Update speed based on status
      let newSpeed = bike.speed;
      if (newStatus === 'idle') {
        newSpeed = 0;
      } else if (newStatus === 'delivering') {
        newSpeed = Math.floor(Math.random() * 20) + 15;
      } else {
        newSpeed = Math.floor(Math.random() * 15) + 10;
      }

      return {
        ...bike,
        longitude: newLng,
        latitude: newLat,
        status: newStatus,
        speed: newSpeed
      };
    });
  }

  /**
   * Get fleet data with automatic polling every 5 seconds
   */
  getFleetDataStream(pollIntervalMs: number = 5000): Observable<FleetData> {
    return interval(pollIntervalMs).pipe(
      startWith(0),
      map(() => {
        this.updateBikePositions();
        return {
          bikes: [...this.bikePositions],
          pollutionZones: this.POLLUTION_ZONES,
          trafficJams: this.TRAFFIC_JAMS,
          timestamp: new Date()
        };
      })
    );
  }

  /**
   * One-time fetch of fleet data
   */
  getFleetData(): Observable<FleetData> {
    this.updateBikePositions();
    return of({
      bikes: [...this.bikePositions],
      pollutionZones: this.POLLUTION_ZONES,
      trafficJams: this.TRAFFIC_JAMS,
      timestamp: new Date()
    });
  }
}
