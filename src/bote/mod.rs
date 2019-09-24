mod protocol;
mod stats;

use crate::bote::stats::Stats;
use crate::planet::{Planet, Region};
use std::collections::HashMap;

pub struct Bote {
    planet: Planet,
}

impl Bote {
    pub fn new(lat_dir: &str) -> Self {
        let planet = Planet::new(lat_dir);
        Bote { planet }
    }

    pub fn run(&self) {
        println!("bote!");
    }

    fn leaderless(
        &self,
        servers: Vec<Region>,
        clients: Vec<Region>,
        quorum_size: usize,
    ) -> Stats {
        let quorum_latencies =
            self.latency_to_closest_quorum(servers.clone(), quorum_size); // TODO can we also avoid this clone?
        let latencies: Vec<_> = clients
            .into_iter()
            .map(|client| {
                // compute the latency from client to the closest available
                // region
                let (closest, client_to_closest) =
                    self.nth_closest(1, &client, &servers);
                // compute the latency from such region to its closest quorum
                let closest_to_quorum = quorum_latencies.get(closest).unwrap();
                // the final latency is the sum of both the above
                client_to_closest + closest_to_quorum
            })
            .collect();
        // compute stats
        Stats::from(&latencies)
    }

    fn leader(
        &self,
        servers: Vec<Region>,
        clients: Vec<Region>,
        quorum_size: usize,
    ) -> HashMap<Region, Stats> {
        let quorum_latencies =
            self.latency_to_closest_quorum(servers.clone(), quorum_size); // TODO can we also avoid this clone?

        servers
            .clone()
            .into_iter()
            .map(|leader| {
                // compute the latency from leader to its closest quorum
                let leader_to_quorum = quorum_latencies.get(&leader).unwrap();

                // compute perceived latency for each client
                let latencies: Vec<_> = clients
                    .iter()
                    .map(|client| {
                        // compute the latency from client to leader
                        let client_to_leader =
                            self.planet.latency(client, &leader).unwrap();
                        // client perceived latency is the sum of both
                        client_to_leader + leader_to_quorum
                    })
                    .collect();

                // compute stats
                let stats = Stats::from(&latencies);

                (leader, stats)
            })
            .collect()
    }

    fn latency_to_closest_quorum(
        &self,
        regions: Vec<Region>,
        quorum_size: usize,
    ) -> HashMap<Region, usize> {
        regions
            .clone() // TODO can we avoid this clone?
            .into_iter()
            .map(|from| {
                let (_, latency) =
                    self.nth_closest(quorum_size, &from, &regions);
                (from, latency)
            })
            .collect()
    }

    /// Compute the latency to the nth closest region.
    /// This same method can be used to find the:
    /// - latency to the closest quorum
    /// - latency to the closest region
    fn nth_closest(
        &self,
        nth: usize,
        from: &Region,
        regions: &Vec<Region>,
    ) -> (&Region, usize) {
        self.planet
            .sorted_by_distance(from)
            .unwrap()
            .into_iter()
            .filter(|(to, _)| regions.contains(to))
            .nth(nth - 1)
            .unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn latency_to_closest_quorum() {
        // create bote
        let lat_dir = "latency/";
        let bote = Bote::new(lat_dir);

        // considered regions
        let w1 = Region::new("europe-west1");
        let w2 = Region::new("europe-west2");
        let w3 = Region::new("europe-west3");
        let w4 = Region::new("europe-west4");
        let w6 = Region::new("europe-west6");
        let regions =
            vec![w1.clone(), w2.clone(), w3.clone(), w4.clone(), w6.clone()];

        // quorum size 2
        let quorum_size = 2;
        let cql = bote.latency_to_closest_quorum(regions.clone(), quorum_size);
        assert_eq!(*cql.get(&w1).unwrap(), 7);
        assert_eq!(*cql.get(&w2).unwrap(), 9);
        assert_eq!(*cql.get(&w3).unwrap(), 7);
        assert_eq!(*cql.get(&w4).unwrap(), 7);
        assert_eq!(*cql.get(&w6).unwrap(), 7);

        // quorum size 3
        let quorum_size = 3;
        let cql = bote.latency_to_closest_quorum(regions.clone(), quorum_size);
        assert_eq!(*cql.get(&w1).unwrap(), 8);
        assert_eq!(*cql.get(&w2).unwrap(), 10);
        assert_eq!(*cql.get(&w3).unwrap(), 7);
        assert_eq!(*cql.get(&w4).unwrap(), 7);
        assert_eq!(*cql.get(&w6).unwrap(), 14);
    }

    #[test]
    fn leaderless() {
        // create bote
        let lat_dir = "latency/";
        let bote = Bote::new(lat_dir);

        // considered regions
        let w1 = Region::new("europe-west1");
        let w2 = Region::new("europe-west2");
        let w3 = Region::new("europe-west3");
        let w4 = Region::new("europe-west4");
        let w6 = Region::new("europe-west6");
        let regions =
            vec![w1.clone(), w2.clone(), w3.clone(), w4.clone(), w6.clone()];

        // quorum size 3
        let quorum_size = 3;
        let stats =
            bote.leaderless(regions.clone(), regions.clone(), quorum_size);
        // w1 -> 9, w2 -> 11, w3 -> 8, w4 -> 8, w6 -> 15
        assert_eq!(stats.mean(), 10);
        assert_eq!(stats.mean_distance_to_mean(), 2);

        // quorum size 4
        let quorum_size = 4;
        let stats =
            bote.leaderless(regions.clone(), regions.clone(), quorum_size);
        // w1 -> 11, w2 -> 14, w3 -> 9, w4 -> 10, w6 -> 15
        assert_eq!(stats.mean(), 11);
        assert_eq!(stats.mean_distance_to_mean(), 2);
    }

    #[test]
    fn leaderless_clients_subset() {
        // create bote
        let lat_dir = "latency/";
        let bote = Bote::new(lat_dir);

        // considered regions
        let w1 = Region::new("europe-west1");
        let w2 = Region::new("europe-west2");
        let w3 = Region::new("europe-west3");
        let w4 = Region::new("europe-west4");
        let w6 = Region::new("europe-west6");
        let servers =
            vec![w1.clone(), w2.clone(), w3.clone(), w4.clone(), w6.clone()];

        // subset of clients: w1 w2
        let clients = vec![w1.clone(), w2.clone()];

        // quorum size 3
        let quorum_size = 3;
        let stats =
            bote.leaderless(servers.clone(), clients.clone(), quorum_size);
        // w1 -> 9, w2 -> 11
        assert_eq!(stats.mean(), 10);
        assert_eq!(stats.mean_distance_to_mean(), 1);

        // quorum size 4
        let quorum_size = 4;
        let stats =
            bote.leaderless(servers.clone(), clients.clone(), quorum_size);
        // w1 -> 11, w2 -> 14
        assert_eq!(stats.mean(), 12);
        assert_eq!(stats.mean_distance_to_mean(), 1);

        // subset of clients: w1 w3 w6
        let clients = vec![w1.clone(), w3.clone(), w6.clone()];

        // quorum size 3
        let quorum_size = 3;
        let stats =
            bote.leaderless(servers.clone(), clients.clone(), quorum_size);
        // w1 -> 9, w3 -> 8, w6 -> 15
        assert_eq!(stats.mean(), 10);
        assert_eq!(stats.mean_distance_to_mean(), 2);

        // quorum size 4
        let quorum_size = 4;
        let stats =
            bote.leaderless(servers.clone(), clients.clone(), quorum_size);
        // w1 -> 11, w3 -> 9, w6 -> 15
        assert_eq!(stats.mean(), 11);
        assert_eq!(stats.mean_distance_to_mean(), 2);
    }

    #[test]
    fn leader() {
        // create bote
        let lat_dir = "latency/";
        let bote = Bote::new(lat_dir);

        // considered regions
        let w1 = Region::new("europe-west1");
        let w2 = Region::new("europe-west2");
        let w3 = Region::new("europe-west3");
        let w4 = Region::new("europe-west4");
        let w6 = Region::new("europe-west6");
        let regions =
            vec![w1.clone(), w2.clone(), w3.clone(), w4.clone(), w6.clone()];

        // quorum size 2:
        let quorum_size = 2;
        let leader_to_stats =
            bote.leader(regions.clone(), regions.clone(), quorum_size);

        // quorum latency for w1 is 7
        // w1 -> 8, w2 -> 17, w3 -> 15, w4 -> 14, w6 -> 21
        let w1_stats = leader_to_stats.get(&w1).unwrap();
        assert_eq!(w1_stats.mean(), 15);
        assert_eq!(w1_stats.mean_distance_to_mean(), 3);

        // quorum latency for w2 is 9
        // w1 -> 19, w2 -> 10, w3 -> 22, w4 -> 18, w6 -> 28
        let w2_stats = leader_to_stats.get(&w2).unwrap();
        assert_eq!(w2_stats.mean(), 19);
        assert_eq!(w2_stats.mean_distance_to_mean(), 4);

        // quorum latency for w3 is 7
        // w1 -> 15, w2 -> 20, w3 -> 8, w4 -> 14, w6 -> 14
        let w3_stats = leader_to_stats.get(&w3).unwrap();
        assert_eq!(w3_stats.mean(), 14);
        assert_eq!(w3_stats.mean_distance_to_mean(), 2);
    }

    #[test]
    fn leader_clients_subset() {
        // create bote
        let lat_dir = "latency/";
        let bote = Bote::new(lat_dir);

        // considered regions
        let w1 = Region::new("europe-west1");
        let w2 = Region::new("europe-west2");
        let w3 = Region::new("europe-west3");
        let w4 = Region::new("europe-west4");
        let w6 = Region::new("europe-west6");
        let servers =
            vec![w1.clone(), w2.clone(), w3.clone(), w4.clone(), w6.clone()];

        // quorum size 2:
        let quorum_size = 2;

        // subset of clients: w1 w2
        let clients = vec![w1.clone(), w2.clone()];
        let leader_to_stats =
            bote.leader(servers.clone(), clients.clone(), quorum_size);

        // quorum latency for w1 is 7
        // w1 -> 8, w2 -> 17
        let w1_stats = leader_to_stats.get(&w1).unwrap();
        assert_eq!(w1_stats.mean(), 12);
        assert_eq!(w1_stats.mean_distance_to_mean(), 4);

        // quorum latency for w2 is 9
        // w1 -> 19, w2 -> 10
        let w2_stats = leader_to_stats.get(&w2).unwrap();
        assert_eq!(w2_stats.mean(), 14);
        assert_eq!(w2_stats.mean_distance_to_mean(), 4);

        // quorum latency for w3 is 7
        // w1 -> 15, w2 -> 20
        let w3_stats = leader_to_stats.get(&w3).unwrap();
        assert_eq!(w3_stats.mean(), 17);
        assert_eq!(w3_stats.mean_distance_to_mean(), 2);

        // subset of clients: w1 w3 w6
        let clients = vec![w1.clone(), w3.clone(), w6.clone()];
        let leader_to_stats =
            bote.leader(servers.clone(), clients.clone(), quorum_size);

        // quorum latency for w1 is 7
        // w1 -> 8, w3 -> 15, w6 -> 21
        let w1_stats = leader_to_stats.get(&w1).unwrap();
        assert_eq!(w1_stats.mean(), 14);
        assert_eq!(w1_stats.mean_distance_to_mean(), 4);

        // quorum latency for w2 is 9
        // w1 -> 19, w3 -> 22, w6 -> 28
        let w2_stats = leader_to_stats.get(&w2).unwrap();
        assert_eq!(w2_stats.mean(), 23);
        assert_eq!(w2_stats.mean_distance_to_mean(), 3);

        // quorum latency for w3 is 7
        // w1 -> 15, w3 -> 8, w6 -> 14
        let w3_stats = leader_to_stats.get(&w3).unwrap();
        assert_eq!(w3_stats.mean(), 12);
        assert_eq!(w3_stats.mean_distance_to_mean(), 3);
    }
}
