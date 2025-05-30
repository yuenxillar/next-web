<template>
  <div class="railway-app">
    <!-- 顶部导航栏 -->
    <a-layout-header class="header">
      <div class="logo-container">
        <img src="@/assets/railway-logo.png" alt="铁路logo" class="logo" />
        <div class="logo-text">
          <div class="main-title">中国铁路12306</div>
          <div class="sub-title">12306 CHINA RAILWAY</div>
        </div>
      </div>
      <a-input-search placeholder="搜索车票、餐饮、旅游、相关问答" class="search-input" :style="{ width: '300px' }">
        <template #enterButton>
          <a-button type="primary">
            <search-outlined />
          </a-button>
        </template>
      </a-input-search>
      <div class="header-right">
        <a-space>
          <a href="#">无障碍</a>
          <a-divider type="vertical" />
          <a href="#">简体版</a>
          <a-divider type="vertical" />
          <a href="#">English</a>
          <a-divider type="vertical" />
          <a href="#">我的12306</a>
          <a-divider type="vertical" />
          <a href="#">登录</a>
          <a href="#">注册</a>
        </a-space>
      </div>
    </a-layout-header>

    <!-- 主导航菜单 -->
    <a-menu mode="horizontal" class="main-menu">
      <a-menu-item key="home" class="menu-item">首页</a-menu-item>
      <a-menu-item key="ticket" class="menu-item">
        车票
        <down-outlined />
      </a-menu-item>
      <a-menu-item key="team" class="menu-item">
        团购服务
        <down-outlined />
      </a-menu-item>
      <a-menu-item key="member" class="menu-item">
        会员服务
        <down-outlined />
      </a-menu-item>
      <a-menu-item key="station" class="menu-item">
        站车服务
        <down-outlined />
      </a-menu-item>
      <a-menu-item key="business" class="menu-item">
        商旅服务
        <down-outlined />
      </a-menu-item>
      <a-menu-item key="travel" class="menu-item">
        出行指南
        <down-outlined />
      </a-menu-item>
      <a-menu-item key="info" class="menu-item">
        信息查询
        <down-outlined />
      </a-menu-item>
    </a-menu>

    <!-- 主体内容 -->
    <div class="main-content">
      <!-- 查询窗口 -->
      <div class="query-container">
        <div class="query-tabs">
          <a-tabs default-active-key="1">
            <a-tab-pane key="1">
              <template #tab>
                <span class="tab-icon"><car-outlined /> 车票</span>
              </template>
              <a-card class="query-card">
                <a-radio-group v-model:value="ticketType" button-style="solid" class="query-type">
                  <a-radio-button value="a">单程</a-radio-button>
                  <a-radio-button value="b">往返</a-radio-button>
                  <a-radio-button value="c">中转换乘</a-radio-button>
                  <a-radio-button value="d">退改签</a-radio-button>
                </a-radio-group>

                <div class="form-item">
                  <div class="label">出发地</div>
                  <a-input placeholder="简拼/全拼/汉字" @focus="hanleFirstInputFocus">
                    <template #suffix>
                      <environment-outlined />
                    </template>
                  </a-input>
                </div>

                <div class="form-item">
                  <div class="label">到达地</div>
                  <a-input placeholder="简拼/全拼/汉字" @focus="hanleFirstInputFocus">
                    <template #suffix>
                      <environment-outlined />
                    </template>
                  </a-input>
                </div>

                <div class="form-item">
                  <div class="label">出发日期</div>
                  <a-date-picker v-model="departureDate" style="width: 100%" :locale="locale" placeholder="请选择日期" />
                </div>

                <div class="form-item">
                  <a-checkbox>学生</a-checkbox>
                  <a-checkbox>高铁/动车</a-checkbox>
                </div>

                <a-button type="primary" block class="query-btn" style="margin-bottom: 30px;">查询</a-button>
              </a-card>
            </a-tab-pane>
            <a-tab-pane key="2">
              <template #tab>
                <span class="tab-icon"><question-circle-outlined /> 常用查询</span>
              </template>
            </a-tab-pane>
            <a-tab-pane key="3">
              <template #tab>
                <span class="tab-icon"><calendar-outlined /> 订餐</span>
              </template>
            </a-tab-pane>
          </a-tabs>
        </div>
      </div>

      <!-- 轮播图 -->
      <div class="banner-container">
        <a-carousel autoplay :after-change="onChange">
          <div v-for="(banner, index) in banners" :key="index">
            <img :src="banner" class="banner-img" />
          </div>
        </a-carousel>
      </div>
    </div>

    <!-- 快捷服务图标 -->
    <div class="quick-services">
      <div class="service-item" @click="goToService('special-passenger')">
        <team-outlined class="service-icon" />
        <div>重点旅客预约</div>
      </div>
      <div class="service-item" @click="goToService('lost-property')">
        <inbox-outlined class="service-icon" />
        <div>遗失物品查找</div>
      </div>
      <div class="service-item" @click="goToService('car-service')">
        <car-outlined class="service-icon" />
        <div>约车服务</div>
      </div>
      <div class="service-item" @click="goToService('consignment')">
        <compass-outlined class="service-icon" />
        <div>便民托运</div>
      </div>
      <div class="service-item" @click="goToService('station-guide')">
        <environment-outlined class="service-icon" />
        <div>车站引导</div>
      </div>
      <div class="service-item" @click="goToService('station-style')">
        <bank-outlined class="service-icon" />
        <div>站车风采</div>
      </div>
      <div class="service-item" @click="goToService('feedback')">
        <user-outlined class="service-icon" />
        <div>用户反馈</div>
      </div>
    </div>

    <!-- 服务卡片区域 -->
    <div class="service-cards">
      <a-row :gutter="16">
        <a-col :span="12">
          <a-card class="service-card">
            <div class="card-image">
              <img src="@/assets/abanner01.jpg" alt="会员服务" />
            </div>
          </a-card>
        </a-col>
        <a-col :span="12">
          <a-card class="service-card">

            <div class="card-image">
              <img src="@/assets/abanner02.jpg" alt="餐饮特产" />
            </div>
          </a-card>
        </a-col>
      </a-row>
      <a-row :gutter="16">
        <a-col :span="12">
          <a-card class="service-card">
            <div class="card-image">
              <img src="@/assets/abanner05.jpg" alt="出行指南" />
            </div>
          </a-card>
        </a-col>
        <a-col :span="12">
          <a-card class="service-card">
            <div class="card-image">
              <img src="@/assets/abanner06.jpg" alt="信息查询" />
            </div>
          </a-card>
        </a-col>
      </a-row>
    </div>
    <div class="new-tab">
      <a-row :gutter="16" class="tab-hd">
        <div v-for="(label, index) in newsTitleList" :key="index" @click="() => { newsIndex = index }"
          :style="{ color: newsIndex === index ? '#fff' : '#666', background: newsIndex === index ? '#3b99fc' : '#efeff4' }">
          {{ label }}</div>
      </a-row>
      <a-row class="tab-bd">
         <div class="tab-item">
           <div class="news-index">
             <ul class="news-index-list">
               <li v-for="item in newsDataList" >
                 <a target="_self"  class="news-tit" style="max-width: 70%;overflow: hidden;text-overflow: ellipsis;white-space: nowrap"
                   :href="item.url" :title="item.title">{{ item.title }}</a>
                 <span class="news-time">{{ item.date }}</span>
               </li>
             </ul>
             <div style="clear: both; width: 100%; line-height: 36px; text-align: right;">
               <a name="g_href" data-type="4" data-href="zxdt/index_zxdt.html" data-redirect="Y" data-target="_blank"
                 style="color: #999;">更多&gt;</a>
             </div>
           </div>
         </div>
       </a-row>
    </div>
    <!-- 底部导航栏 -->
    <div class="footer">
      <div class="footer-con">
        <div class="footer-links">
          <a-row>
            <h2 style="font-size: 15px;">友情连接</h2>
          </a-row>
          <a-row :gutter="8">
            <a-col>
              <a name="g_href" title="中国国家铁路集团有限公司" data-href="http://www.china-railway.com.cn/" data-redirect="N"
                href="javascript:;" data-target="_blank">
                <img src="@/assets/links/link05.png" alt="中国国家铁路集团有限公司">
              </a>
            </a-col>
            <a-col>
              <a name="g_href" title="中国铁路财产保险自保有限公司" data-href="http://www.china-ric.com/" data-redirect="N"
                href="javascript:;" data-target="_blank">
                <img src="@/assets/links/link02.png" alt="中国铁路财产保险自保有限公司">
              </a>
            </a-col>
          </a-row>
          <a-row :gutter="8">
            <a-col>
              <a name="g_href" title="中国铁路95306网" data-href="http://www.95306.cn/" data-redirect="N" href="javascript:;"
                data-target="_blank">
                <img src="@/assets/links/link03.png" alt="中国铁路95306网">
              </a>
            </a-col>
            <a-col>
              <a name="g_href" title="中铁快运股份有限公司" data-href="http://www.95572.com/" data-redirect="N"
                href="javascript:;" data-target="_blank">
                <img src="@/assets/links/link04.png" alt="中铁快运股份有限公司">
              </a>
            </a-col>
          </a-row>
        </div>
        <div class="foot-code">
          <a-row :gutter="40">
            <a-col>
              <h2 style="font-size: 15px;">中国铁路官方微信</h2>
              <img src="@/assets/code/zgtlwx.png" alt="中国铁路官方微信">
            </a-col>
            <a-col>
              <h2 style="font-size: 15px;">中国铁路官方微博</h2>
              <img src="@/assets/code/zgtlwb.png" alt="中国铁路官方微博">
            </a-col>
            <a-col>
              <h2 style="font-size: 15px;">12306 公众号</h2>
              <img src="@/assets/code/public.png" alt="12306 公众号">
            </a-col>
            <a-col>
              <h2 style="font-size: 15px;">铁路12306</h2>
              <img src="@/assets/code/download.png" alt="铁路12306">
            </a-col>
          </a-row>
        </div>

        <div class="code-tips">
          官方APP下载，目前铁路未授权其他网站或APP开展类似服务内容，敬请广大用户注意。
        </div>
      </div>
      <br>
      <br>
      <div class="footer-txt">
        <a-row class="footer-warpper">
          <p class="footer-text">
            <span class="mr">版权所有©2008-2025</span>
            <span class="mr">中国铁道科学研究院集团有限公司</span>
            <span class="mr">技术支持：铁旅科技有限公司</span>
          </p>
        </a-row>
        <a-row class="footer-warpper">
          <p class="footer-text">
            <span><img src="@/assets/gongan.png" alt="公安" style="margin-right: 4px;width: 14px;" /></span>
            <span class="mr">京公网安备 11010802038392号</span>
            <span class="mr">|</span>
            <span class="mr">京ICP备05020493号-4</span>
            <span class="mr">|</span>
            <span class="mr">ICP证：京B2-20202537</span>
            <span class="mr">|</span>
            <span class="mr">营业执照</span>
          </p>
        </a-row>
        <div class="footer-slh">
          <img src="@/assets/footer-slh.jpg" alt="适老化无障碍服务">
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import zhCN from 'ant-design-vue/es/locale/zh_CN';
import {
  SearchOutlined,
  DownOutlined,
  CarOutlined,
  QuestionCircleOutlined,
  CalendarOutlined,
  TeamOutlined,
  InboxOutlined,
  CompassOutlined,
  EnvironmentOutlined,
  BankOutlined,
  UserOutlined
} from '@ant-design/icons-vue';

const ticketType = ref('a');
const newsTitleList = ref([
  '最新发布',
  '常见问题',
  '信用信息',
]);
const newsDataList = ref([
  {
    "title": "公 告",
    "date": "2024-12-11",
    "url": "http://www.12306.cn/mormhweb/zxdt/202412/t20241211_43192.html"
  },
  {
    "title": "关于优化铁路车票改签规则的公告",
    "date": "2024-01-11",
    "url": "http://www.12306.cn/mormhweb/zxdt/202412/t20241211_43192.html"
  },
  {
    "title": "铁路旅客禁止、限制携带和托运物品目录",
    "date": "2023-11-30",
    "url": "http://www.12306.cn/mormhweb/zxdt/202412/t20241211_43192.html"
  },
  {
    "title": "公 告",
    "date": "2022-12-22",
    "url": "http://www.12306.cn/mormhweb/zxdt/202412/t20241211_43192.html"
  },
  {
    "title": "中国铁路上海局集团有限公司关于2025年5月30日-2025年6月30日增开部分旅客列车的公告",
    "date": "2025-05-27",
    "url": "http://www.12306.cn/mormhweb/zxdt/202412/t20241211_43192.html"
  },
  {
    "title": "关于铁路客运推广使用全面数字化的电子发票的公告",
    "date": "2024-11-07",
    "url": "http://www.12306.cn/mormhweb/zxdt/202412/t20241211_43192.html"
  },
  {
    "title": "外国护照身份核验使用说明",
    "date": "2023-12-13",
    "url": "http://www.12306.cn/mormhweb/zxdt/202412/t20241211_43192.html"
  },
  {
    "title": "候补购票操作说明",
    "date": "2024-04-19",
    "url": "http://www.12306.cn/mormhweb/zxdt/202412/t20241211_43192.html"
  },
  {
    "title": "关于调整互联网、电话订票起售时间的公告",
    "date": "2025-03-29",
    "url": "http://www.12306.cn/mormhweb/zxdt/202412/t20241211_43192.html"
  },
  {
    "title": "中国铁路呼和浩特局集团有限公司关于2025年5月27停运部分旅客列车的公告",
    "date": "2025-05-26",
    "url": "http://www.12306.cn/mormhweb/zxdt/202412/t20241211_43192.html"
  }
])

const newsIndex = ref(0); // 当前新闻标题索引

const locale = zhCN;
const departureDate = ref<any>();
const banners = [
  '/banner10.jpg',
  '/banner12.jpg',
  '/banner26.jpg',
  '/banner0619.jpg',
  '/banner20200707.jpg',
  '/banner20201223.jpg',
];


// banner 切换事件处理函数
const onChange = (current: number) => {
};

const hanleFirstInputFocus = () => {
  console.log('focus');
}

const goToService = (serviceType: string) => {
  const uris = {
    'special-passenger': 'view/icentre_qxyyInfo.html',
    'lost-property': 'view/icentre_lostInfo.html',
    'car-service': 'view/station/shared_Car.html',
    'consignment': 'view/station/hand.html',
    'station-guide': 'czyd_2143/',
    'station-style': 'zcfc_2548/',
    'feedback': 'view/advice_adviceInfo.html'
  };
  // 为了避免类型错误，先检查 serviceType 是否是 uris 对象的有效键
  // @ts-ignore
  const url = `https://www.12306.cn/index/${uris[serviceType]}`;
  window.open(url, '_blank');
}
</script>

<style scoped>
.railway-app {
  font-family: 'Microsoft YaHei', Arial, sans-serif;
  width: 100%;
  min-height: 100vh;
  margin: 0;
  padding: 0;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  align-items: center;
  background-color: white;
}

/* 添加全局重置样式 */
html,
body {
  margin: 0;
  padding: 0;
  width: 100%;
  height: 100%;
  overflow-x: hidden;
}

/* 修改 header 样式确保宽度 100% */
.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 60px;
  padding: 0 20px;
  background-color: white;
  width: 70%;
}

.logo-container {
  display: flex;
  align-items: center;
}

.logo {
  width: 40px;
  height: 40px;
  margin-right: 10px;
}

.logo-text {
  display: flex;
  flex-direction: column;
}

.main-title {
  font-size: 16px;
  font-weight: bold;
  color: #333;
}

.sub-title {
  font-size: 12px;
  color: #999;
}

.header-right {
  font-size: 12px;
}

/* 确保菜单宽度 100% */
.main-menu {
  background-color: #4a90e2;
  color: white;
  display: flex;
  justify-content: center;
  width: 100%;
}

.menu-item {
  color: white !important;
  font-size: 15px;
  padding-right: 100px;
}

/* 确保内容区域宽度 100% */
.main-content {
  display: flex;
  min-height: 360px;
  width: 100%;
}

.query-container {
  position: absolute;
  width: 500px;
  padding: 20px;
  padding-top: 10px;
  z-index: 100;
  left: 20vw;
  max-height: 160px;
}

.query-tabs {
  background-color: white;
  border-radius: 4px;
  align-items: center;
  justify-items: center;
}

.tab-icon {
  display: flex;
  align-items: center;
  gap: 5px;
}

.query-card {
  border: none;
  box-shadow: none;
}

.query-type {
  margin-bottom: 10px;
  width: 100%;
  display: flex;
}

.form-item {
  margin-bottom: 15px;
}

.label {
  margin-bottom: 5px;
  font-size: 14px;
  justify-self: start;
}

.query-btn {
  height: 40px;
  font-size: 16px;
}

.banner-container {
  flex: 1;
  position: relative;
  width: 100%;
  height: 100%;
  min-height: 30vh;
}

.banner {
  height: 100%;
  width: 100%;
}

.banner-img {
  width: 100%;
  height: 100%;
  min-height: 450px;
  object-fit: cover;
  transition: all 0.5s ease;
}

.province {
  font-size: 80px;
  margin-bottom: 10px;
}

.slogan {
  font-size: 36px;
  margin-bottom: 10px;
}

.description {
  font-size: 24px;
}

.carousel-indicators {
  position: absolute;
  bottom: 20px;
  width: 100%;
  text-align: center;
}

.quick-services {
  display: flex;
  justify-content: space-around;
  padding: 20px 0;
  background-color: white;
  width: 80%;
  min-height: 10vh;
}

.service-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  font-size: 14px;
  color: #666;
  cursor: pointer;
}

.service-icon {
  font-size: 24px;
  color: #4a90e2;
  margin-bottom: 8px;

}

.service-item:hover {
  opacity: 0.8;
}

.service-cards {
  padding: 20px;
  max-width: 76%;
}

.new-tab {
  padding: 0;
  min-width: 75%;
  max-width: 75%;
}

.tab-hd {
  display: flex;
  justify-content: space-between;
  /* align-items: center; */
  justify-self: center;
  width: 100%;
}

.tab-hd div {
  background: #efeff4;
  color: #666;
  display: block;
  /* min-width: 400px; */
  width: 32%;
  height: 50px;
  line-height: 40px;
  font-size: 18px;
  align-content: center;
  cursor: pointer;
}

.tab-bd {
  border: 1px solid #dedede;
  width:  100%;
  height: 30vh;
  min-height: 300px;
  padding: 0;
  margin: 0;
}

.tab-item {
  padding: 20px;
  width: 100%;
  height: 100%;
  overflow: hidden;
}

.news-index {
  overflow: hidden;
}

.news-index li {
  list-style: none;
  float: left;
  width: 544px;
  margin-right: 100px;
  height: 36px;
  line-height: 36px;
}

.news-index a {
  color: #333333;
  font-size: 16px;
}

.news-index a:hover {
  color: #000;
}

.news-index-list {
  display: block;
  list-style-type: disc;
  margin-block-start: 1em;
  margin-block-end: 1em;
  padding-inline-start: 40px;
  unicode-bidi: isolate;
}

.news-index li:before {
  content: "";
  float: left;
  width: 6px;
  height: 6px;
  background: #3b99fc;
  margin: 15px 20px 0 0;
}

.news-index ul::before {
  content: "";
  display: table;
}

.news-tit {
  float: left;
  height: 36px;
  line-height: 36px;
  overflow: hidden;
  -ms-text-overflow: ellipsis;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.news-time {
  float: right;
  color: #999;
  font-size: 14px;
}

.service-card {
  margin-bottom: 20px;
  border-radius: 6px;
  overflow: hidden;
  cursor: pointer;
}

.card-image {
  flex: 1;
  text-align: right;
  transition: 0.3s linear opacity;
}

.card-image:hover {
  opacity: 0.8;
}


:deep(.ant-card-body) {
  /* 样式规则 */
  padding: 0px;
}

.card-image img {
  height: 100%;
  width: 100%;
  object-fit: cover;
  transition: 0.3s linear opacity;
}

.footer {
  width: 100%;
  height: 40%;
  background: #f8f8f8;
  ;
}

.footer-con {
  width: 70%;
  height: 50%;
  display: flex;
  justify-self: center;
  justify-content: center;
  box-sizing: border-box;
  margin-top: 20px;
}

.footer-links {
  width: 40%;
  height: 100%;
  justify-self: center;
}

.foot-code {
  width: 40%;
  height: 100%;
  justify-self: center;
  text-align: center;
}

.foot-code img {
  display: block;
  width: 80px;
  height: 80px;
  border: 1px solid #dedede;
}

.footer-links img {
  display: block;
  width: 220px;
  height: 37px;
  border: 1px solid #dedede;
  margin-bottom: 10px;
}

.code-tips {
  width: 190px;
  height: 80px;
  border: 1px solid #dedede;
  background-color: #fff;
  background-position: right bottom;
  background-repeat: no-repeat;
  line-height: 18px;
  padding: 12px 10px;
  font-size: 12px;
  text-align: left;
  margin-left: 20px;
  align-self: center;
  justify-self: center;
  margin-top: 12px;
}

.footer-txt {
  align-content: center;
  align-items: center;
  min-height: 100px;
  width: 100%;
  background-color: #666666;
}

.footer-warpper {
  display: relative;
  justify-content: center;
  text-align: center;
}

.footer-text {
  color: #c1c1c1;
  text-align: center;
  font-size: 14.5px;
}

.mr {
  margin-right: 10px;
}

.footer-slh {
  position: absolute;
  left: 50%;
  margin-left: 465px;
}

.footer-slh img {
  display: block;
  width: 150px;
  height: 50px;
}
</style>