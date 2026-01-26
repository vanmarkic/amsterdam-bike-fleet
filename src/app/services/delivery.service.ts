import { Injectable } from '@angular/core';
import { HttpClient, HttpParams } from '@angular/common/http';
import { Observable } from 'rxjs';
import { Delivery, DeliveryStatus } from '../models/fleet.models';

export interface DeliveryFilters {
  status?: DeliveryStatus;
  bikeId?: string;
}

@Injectable({
  providedIn: 'root'
})
export class DeliveryService {
  private readonly API_URL = '/api/deliveries';

  constructor(private http: HttpClient) {}

  getDeliveries(filters?: DeliveryFilters): Observable<Delivery[]> {
    let params = new HttpParams();

    if (filters?.status) {
      params = params.set('status', filters.status);
    }
    if (filters?.bikeId) {
      params = params.set('bikeId', filters.bikeId);
    }

    return this.http.get<Delivery[]>(this.API_URL, { params });
  }

  getDeliveryById(id: string): Observable<Delivery> {
    return this.http.get<Delivery>(`${this.API_URL}/${id}`);
  }
}
