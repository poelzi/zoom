pub mod basic_particle;
pub use self::basic_particle::*;

extern crate num;
use self::num::Float;
use super::vector::*;

//An object that has quanta
pub trait Quanta<D> {
    //Retrieve the quanta of a physics particle
    fn quanta(&self) -> D;
}

//An object that has inertia
pub trait Inertia<D> {
    //Retrieve the inertia of a physics particle
    fn inertia(&self) -> D;
}

//An object with a location
pub trait Position<V> {
    //Get position of particle
    fn position(&self) -> V;
}

//An object with a velocity
pub trait Velocity<V> {
    //Get velocity of particle
    fn velocity(&self) -> V;
}

//A uniform sphere with a radius
pub trait UniformBall<D> {
    //Get radius of particle
    fn radius(&self) -> D;
    //Get radius squared of particle (override this when more efficient)
    fn radius_squared(&self) -> D
        where D: Float
    {
        self.radius().powi(2)
    }
}

//An object that has a simple particle motion interface
pub trait Particle<V, D>: Position<V> + Velocity<V> + Inertia<D> {
    //Accelerate particle
    fn accelerate(&mut self, vec: &V);
    //Advance particle (update position and velocity)
    fn advance(&mut self, time: D);
}

pub trait PhysicsParticle<V, D>: Particle<V, D> + Quanta<D> + Inertia<D>
    where V: Vector<D>, D: Float
{
    //Convert a PhysicsParticle into a "basic particle" that implements PhysicsParticle but has the minimum members
    fn basic_form(&self) -> BasicParticle<V, D> {
        BasicParticle::new(self.quanta(), self.position(), self.velocity(), self.inertia())
    }

    //Apply drag forces to a particle
    fn drag(&mut self, magnitude: D) {
        let acceleration = -self.velocity() * magnitude / self.inertia();
        self.accelerate(&acceleration);
    }

    //Apply proper attraction between two physics particles based on their quanta and position
    fn gravitate<T: ?Sized>(lhs: &mut Self, rhs: &mut T, magnitude: D)
        where T: PhysicsParticle<V, D>
    {
        //Create delta vector between the two positions
        let delta = rhs.position() - lhs.position();
        //Find the distance of the delta vector
        let distance = delta.displacement();
        //Dividing the delta vector by the distance cubed computes attractive force as per the inverse square law.
        //The extra degree normalizes the direction vector to a unit vector.
        let force = delta * magnitude / distance.powi(3) *
        //Multiply by the two quanta of the objects to compute the final force.
        lhs.quanta() * rhs.quanta();

        //Accelerate lhs in the direction of force and divide it by the inertia
        let acceleration = force / lhs.inertia();
        lhs.accelerate(&acceleration);
        //Apply the inverse force to the rhs similarly
        let acceleration = -force / rhs.inertia();
        rhs.accelerate(&acceleration);
    }

    /*
    If gravitating at a distance greater than the radius, then gravitational interaction is applied as if the
    particles are point particles. If the distance is less than the radius, then the interaction happens as if the
    gravitational quanta (mass) is evenly distributed and gravitational flux is used instead, which causes the
    interaction to become proportional to the radius, meaning that as the radius approaches zero, so does the force.

    The radius is passed squared for internal efficiency reasons.
    */
    fn gravitate_radius_squared<T: ?Sized>(lhs: &mut Self, rhs: &mut T, radius_squared: D, magnitude: D)
        where T: PhysicsParticle<V, D>
    {
        let delta = rhs.position() - lhs.position();
        let distance_squared = delta.displacement_squared();
        //Force is the only thing that changes from normal gravitation
        let force = delta * magnitude * lhs.quanta() * rhs.quanta() / if distance_squared > radius_squared {
            distance_squared.sqrt().powi(3)
        } else {
            radius_squared
        };
        let acceleration = force / lhs.inertia();
        lhs.accelerate(&acceleration);
        let acceleration = -force / rhs.inertia();
        rhs.accelerate(&acceleration);
    }

    //This function exists so that if an optimization is possible later, it can be specially implemented.
    //Presently it just acts as a frontend to its squared counterpart. Prefer squared version if possible.
    fn gravitate_radius<T: ?Sized>(lhs: &mut Self, rhs: &mut T, radius: D, magnitude: D)
        where T: PhysicsParticle<V, D>
    {
        Self::gravitate_radius_squared(lhs, rhs, radius * radius, magnitude);
    }

    //Apply proper attraction to a single physics particle towards a location and with a magnitude
    fn gravitate_to<T: ?Sized>(&mut self, center: &T, magnitude: D)
        where T: Quanta<D> + Position<V>
    {
        //Create delta vector from the particle to the center of attraction
        let delta = center.position() - self.position();
        let distance = delta.displacement();
        let acceleration = delta / distance.powi(3) * magnitude *
            self.quanta() * center.quanta() / self.inertia();
        self.accelerate(&acceleration);
    }

    //Works the same as gravitate_radius_squared and gravitate_to
    fn gravitate_radius_to<T: ?Sized>(&mut self, center: &T, magnitude: D)
        where T: Quanta<D> + Position<V> + UniformBall<D>
    {
        //Create delta vector from the particle to the center of attraction
        let delta = center.position() - self.position();
        let distance_squared = delta.displacement_squared();
        let acceleration = delta * magnitude * self.quanta() * center.quanta() / (
                self.inertia() *
                if distance_squared > center.radius_squared() {
                    distance_squared.sqrt().powi(3)
                } else {
                    center.radius_squared()
                }
            );
        self.accelerate(&acceleration);
    }

    //Apply spring forces between two particles
    fn hooke<T: ?Sized>(lhs: &mut Self, rhs: &mut T, magnitude: D)
        where T: PhysicsParticle<V, D>
    {
        let delta = rhs.position() - lhs.position();
        let force = delta.normalized() * magnitude * delta.displacement() * lhs.quanta() * rhs.quanta();
        let acceleration = force / lhs.inertia();
        lhs.accelerate(&acceleration);
        let acceleration = -force / rhs.inertia();
        rhs.accelerate(&acceleration);
    }

    //Apply spring forces between two particles with specified equilibrium distance
    fn hooke_equilibrium<T: ?Sized>(lhs: &mut Self, rhs: &mut T, equilibrium: D, magnitude: D)
        where T: PhysicsParticle<V, D>
    {
        let delta = rhs.position() - lhs.position();
        let force = delta.normalized() * magnitude *
            //We scale such that if displacement is greater than equilibrium, the particles attract proportionally
            //Particles with a displacement smaller than equilibrium repel each other
            (delta.displacement() - equilibrium) *
            lhs.quanta() * rhs.quanta();
        let acceleration = force / lhs.inertia();
        lhs.accelerate(&acceleration);
        let acceleration = -force / rhs.inertia();
        rhs.accelerate(&acceleration);
    }

    //Apply spring forces between one particle and a virtual particle that is unaffected
    fn hooke_to<T: ?Sized>(&mut self, center: &T, magnitude: D)
        where T: Quanta<D> + Position<V>
    {
        let delta = center.position() - self.position();
        let acceleration = delta.normalized() * delta.displacement() *
            magnitude * self.quanta() * center.quanta() / self.inertia();
        self.accelerate(&acceleration);
    }

    //Apply spring forces between one particle and a virtual particle that is unaffected
    fn hooke_equilibrium_to<T: ?Sized>(&mut self, center: &T, equilibrium: D, magnitude: D)
        where T: Quanta<D> + Position<V>
    {
        let delta = center.position() - self.position();
        let acceleration = delta.normalized() * (delta.displacement() - equilibrium) *
            magnitude * self.quanta() * center.quanta() / self.inertia();
        self.accelerate(&acceleration);
    }

    //Apply lorentz forces between two PhysicsParticle objects based on quanta, position, and velocity
    fn lorentz<T: ?Sized>(lhs: &mut Self, rhs: &mut T, magnitude: D)
        where V: CrossVector, T: PhysicsParticle<V, D>
    {
        let delta = rhs.position() - lhs.position();
        let distance = delta.displacement();
        let force = V::cross(&rhs.velocity(), &V::cross(&lhs.velocity(), &delta)) *
            lhs.quanta() * rhs.quanta() / distance.powi(3) * magnitude;

        let acceleration = -force / lhs.inertia();
        lhs.accelerate(&acceleration);
        let acceleration = force / rhs.inertia();
        rhs.accelerate(&acceleration);
    }

    //Apply lorentz forces between two PhysicsParticle objects with a radius_squared
    fn lorentz_radius_squared<T: ?Sized>(lhs: &mut Self, rhs: &mut T, radius_squared: D, magnitude: D)
        where V: CrossVector, T: PhysicsParticle<V, D>
    {
        let delta = rhs.position() - lhs.position();
        let distance_squared = delta.displacement_squared();
        let force = V::cross(&rhs.velocity(), &V::cross(&lhs.velocity(), &delta)) * magnitude *
            lhs.quanta() * rhs.quanta() /
            if distance_squared > radius_squared {
                distance_squared.sqrt().powi(3)
            } else {
                radius_squared
            };

        let acceleration = -force / lhs.inertia();
        lhs.accelerate(&acceleration);
        let acceleration = force / rhs.inertia();
        rhs.accelerate(&acceleration);
    }

    //This function exists so that if an optimization is possible later, it can be specially implemented.
    //Presently it just acts as a frontend to its squared counterpart. Prefer squared version if possible.
    fn lorentz_radius<T: ?Sized>(lhs: &mut Self, rhs: &mut T, radius: D, magnitude: D)
        where V: CrossVector, T: PhysicsParticle<V, D>
    {
        Self::lorentz_radius_squared(lhs, rhs, radius * radius, magnitude);
    }

    //Apply lorentz force to a particle in a field given by a vector with the magnitude and direction of the field
    fn lorentz_field(&mut self, field: &V)
        where V: CrossVector
    {
        let acceleration = V::cross(&self.velocity(), field) * self.quanta() / self.inertia();
        self.accelerate(&acceleration);
    }

    //Apply the lorentz force on a virtual particle that is unaffected
    fn lorentz_to<T: ?Sized>(lhs: &mut Self, center: &T, magnitude: D)
        where V: CrossVector, T: Quanta<D> + Position<V> + Velocity<V> + UniformBall<D>
    {
        let delta = center.position() - lhs.position();
        let distance_squared = delta.displacement_squared();
        let force = V::cross(&center.velocity(), &V::cross(&lhs.velocity(), &delta)) * magnitude *
            lhs.quanta() * center.quanta() /
            if distance_squared > center.radius_squared() {
                distance_squared.sqrt().powi(3)
            } else {
                center.radius_squared()
            };

        let acceleration = -force / lhs.inertia();
        lhs.accelerate(&acceleration);
    }
}
