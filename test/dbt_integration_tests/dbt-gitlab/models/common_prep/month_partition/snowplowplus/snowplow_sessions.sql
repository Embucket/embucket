

WITH snowplow_sessions as (

    select * from {{ ref('snowplow_sessions_tmp') }}

),

id_map as (

    select * from {{ ref('snowplow_id_map') }}

),

stitched as (

    select
        user_custom_id,
        coalesce(id.user_id, user_snowplow_domain_id) as inferred_user_id,
        user_snowplow_domain_id,
        user_snowplow_crossdomain_id,

        app_id,
        browser,
        browser_build_version,
        browser_engine,
        browser_language,
        browser_major_version,
        browser_minor_version,
        browser_name,
        device,
        device_is_mobile,
        device_type,
        first_page_title,
        first_page_url,
        first_page_url_fragment,
        first_page_url_host,
        first_page_url_path,
        first_page_url_port,
        first_page_url_query,
        first_page_url_scheme,
        exit_page_url,
        geo_city,
        geo_country,
        geo_latitude,
        geo_longitude,
        geo_region,
        geo_region_name,
        geo_timezone,
        geo_zipcode,
        ip_address,
        ip_domain,
        ip_isp,
        ip_net_speed,
        ip_organization,
        marketing_campaign,
        marketing_click_id,
        marketing_content,
        marketing_medium,
        marketing_network,
        marketing_source,
        marketing_term,
        os,
        os_build_version,
        os_major_version,
        os_manufacturer,
        os_minor_version,
        os_name,
        os_timezone,
        page_views,
        referer_medium,
        referer_source,
        referer_term,
        referer_url,
        referer_url_fragment,
        referer_url_host,
        referer_url_path,
        referer_url_port,
        referer_url_query,
        referer_url_scheme,
        session_start,
        session_start_local,
        session_end,
        session_end_local,
        session_id,
        session_index as session_cookie_index,
        time_engaged_in_s,
        time_engaged_in_s_tier,
        user_bounced
        , first_glm_source
        , last_glm_source
        
        , first_gsc_environment
        , last_gsc_environment
        
        , first_gsc_extra
        , last_gsc_extra
        
        , first_gsc_namespace_id
        , last_gsc_namespace_id
        
        , first_gsc_plan
        , last_gsc_plan
        
        , first_gsc_google_analytics_client_id
        , last_gsc_google_analytics_client_id
        
        , first_gsc_project_id
        , last_gsc_project_id
        
        , first_gsc_pseudonymized_user_id
        , last_gsc_pseudonymized_user_id
        
        , first_gsc_source
        , last_gsc_source
        
        , first_gsc_is_gitlab_team_member
        , last_gsc_is_gitlab_team_member
        
        , first_cf_formid
        , last_cf_formid
        
        , first_cf_elementid
        , last_cf_elementid
        
        , first_cf_nodename
        , last_cf_nodename
        
        , first_cf_type
        , last_cf_type
        
        , first_cf_elementclasses
        , last_cf_elementclasses
        
        , first_cf_value
        , last_cf_value
        
        , first_sf_formid
        , last_sf_formid
        
        , first_sf_formclasses
        , last_sf_formclasses
        
        , first_sf_elements
        , last_sf_elements
        
        , first_ff_formid
        , last_ff_formid
        
        , first_ff_elementid
        , last_ff_elementid
        
        , first_ff_nodename
        , last_ff_nodename
        
        , first_ff_elementtype
        , last_ff_elementtype
        
        , first_ff_elementclasses
        , last_ff_elementclasses
        
        , first_ff_value
        , last_ff_value
        
        , first_lc_elementid
        , last_lc_elementid
        
        , first_lc_elementclasses
        , last_lc_elementclasses
        
        , first_lc_elementtarget
        , last_lc_elementtarget
        
        , first_lc_targeturl
        , last_lc_targeturl
        
        , first_lc_elementcontent
        , last_lc_elementcontent
        
        , first_tt_category
        , last_tt_category
        
        , first_tt_variable
        , last_tt_variable
        
        , first_tt_timing
        , last_tt_timing
        
        , first_tt_label
        , last_tt_label

    from snowplow_sessions as s
    left outer join id_map as id on s.user_snowplow_domain_id = id.domain_userid

)

select
    *,
    row_number() over (partition by inferred_user_id order by session_start) as session_index

from stitched