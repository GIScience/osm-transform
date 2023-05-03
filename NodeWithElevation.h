#ifndef OSM_TRANSFORM_NODEWITHELEVATION_H
#define OSM_TRANSFORM_NODEWITHELEVATION_H

#include <osmium/osm/location.hpp>


#include <cstdint>
#include "osmtypes.hpp"


class NodeWithElevation {
    osmid_t m_id;
    int32_t m_x; // NOLINT(modernize-use-default-member-init)
    int32_t m_y; // NOLINT(modernize-use-default-member-init)
    int32_t m_z;

    constexpr static double precision() noexcept {
        return static_cast<double>(osmium::detail::coordinate_precision);
    }

public:

    explicit constexpr NodeWithElevation() noexcept :
            m_id(-1),
            m_x(osmium::Location::undefined_coordinate),
            m_y(osmium::Location::undefined_coordinate),
            m_z(0) {
    }

    constexpr NodeWithElevation(const osmid_t id, const int32_t x, const int32_t y, const int32_t z) noexcept :
            m_id(id),
            m_x(x),
            m_y(y),
            m_z(z) {
    }

    constexpr osmid_t id() const noexcept {
        return m_id;
    }

    constexpr int32_t x() const noexcept {
        return m_x;
    }

    constexpr int32_t y() const noexcept {
        return m_y;
    }

    constexpr int32_t elevation() const noexcept {
        return m_z;
    }

    double lon() const {
        if (!valid()) {
            throw osmium::invalid_location{"invalid location"};
        }
        return osmium::Location::fix_to_double(m_x);
    }

    double lat() const {
        if (!valid()) {
            throw osmium::invalid_location{"invalid location"};
        }
        return osmium::Location::fix_to_double(m_y);
    }

    constexpr bool valid() const noexcept {
        return m_x >= -180 * precision()
               && m_x <=  180 * precision()
               && m_y >=  -90 * precision()
               && m_y <=   90 * precision();
    }
};


#endif //OSM_TRANSFORM_NODEWITHELEVATION_H
